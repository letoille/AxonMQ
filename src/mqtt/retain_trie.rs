use std::collections::{BTreeSet, HashMap};

use bytes::Bytes;

use crate::mqtt::{QoS, protocol::property::Property};

#[derive(Clone)]
pub struct RetainedMessage {
    pub topic: String,
    pub qos: QoS,
    pub payload: Bytes,
    pub properties: Vec<Property>,
    pub expiry_at: Option<u64>,
}

#[derive(Default, Clone)]
pub struct RetainedTrieNode {
    pub message: Option<RetainedMessage>,
    pub children: HashMap<String, RetainedTrieNode>,
}

impl RetainedTrieNode {
    fn is_empty(&self) -> bool {
        self.message.is_none() && self.children.is_empty()
    }
}

#[derive(Default)]
pub struct RetainedTrie {
    root: RetainedTrieNode,
    expiry_index: BTreeSet<(u64, String)>, // (expire_at, topic)
}

impl RetainedTrie {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, topic: &str, message: RetainedMessage) {
        if let Some(old_msg) = self.get_message_mut(topic) {
            let old_expiry = old_msg.expiry_at;
            *old_msg = message;
            if let Some(old_expire) = old_expiry {
                self.expiry_index.remove(&(old_expire, topic.to_string()));
            }
        } else {
            let mut current_node = &mut self.root;
            for part in topic.split('/') {
                current_node = current_node.children.entry(part.to_string()).or_default();
            }
            current_node.message = Some(message);
        }
        let msg_ref = self.get_message(topic).unwrap();
        if let Some(expire_at) = msg_ref.expiry_at {
            self.expiry_index.insert((expire_at, topic.to_string()));
        }
    }

    fn recursive_remove(node: &mut RetainedTrieNode, topic_parts: &[&str]) -> bool {
        if let Some((current_part, remaining_parts)) = topic_parts.split_first() {
            let part_string = current_part.to_string();
            let mut child_became_empty = false;

            if let Some(mut child) = node.children.get_mut(&part_string) {
                if Self::recursive_remove(&mut child, remaining_parts) {
                    child_became_empty = true;
                }
            }
            if child_became_empty {
                node.children.remove(&part_string);
            }
        } else {
            node.message = None;
        }
        node.is_empty()
    }

    pub fn remove(&mut self, topic: &str) {
        if let Some(msg) = self.get_message(topic) {
            if let Some(expiry) = msg.expiry_at {
                self.expiry_index.remove(&(expiry, topic.to_string()));
            }
        }
        let parts: Vec<&str> = topic.split('/').collect();
        Self::recursive_remove(&mut self.root, &parts);
    }

    pub fn find_matches_for_filter(&self, filter: &str) -> Vec<&RetainedMessage> {
        let mut results = Vec::new();
        let filter_parts: Vec<&str> = filter.split('/').collect();
        self.recursive_find(&self.root, &filter_parts, &mut results);
        results
    }

    fn recursive_find<'a>(
        &self,
        current_node: &'a RetainedTrieNode,
        filter_parts: &[&str],
        results: &mut Vec<&'a RetainedMessage>,
    ) {
        if let Some((current_filter, remaining_filter)) = filter_parts.split_first() {
            if *current_filter == "#" {
                self.collect_all_messages(current_node, results);
                return;
            }

            if *current_filter == "+" {
                for child_node in current_node.children.values() {
                    self.recursive_find(child_node, remaining_filter, results);
                }
            } else {
                if let Some(child_node) = current_node.children.get(*current_filter) {
                    self.recursive_find(child_node, remaining_filter, results);
                }
            }
        } else {
            if let Some(msg) = &current_node.message {
                results.push(msg);
            }
        }
    }

    fn collect_all_messages<'a>(
        &self,
        node: &'a RetainedTrieNode,
        results: &mut Vec<&'a RetainedMessage>,
    ) {
        if let Some(msg) = &node.message {
            results.push(msg);
        }
        for child in node.children.values() {
            self.collect_all_messages(child, results);
        }
    }

    pub fn get_message(&self, topic: &str) -> Option<&RetainedMessage> {
        let mut current_node = &self.root;
        for part in topic.split('/') {
            if let Some(node) = current_node.children.get(part) {
                current_node = node;
            } else {
                return None;
            }
        }
        current_node.message.as_ref()
    }

    fn get_message_mut(&mut self, topic: &str) -> Option<&mut RetainedMessage> {
        let mut current_node = &mut self.root;
        for part in topic.split('/') {
            current_node = current_node.children.entry(part.to_string()).or_default();
        }
        current_node.message.as_mut()
    }

    pub fn purge_expired(&mut self) {
        let expired_topics: Vec<String> = self
            .expiry_index
            .iter()
            .take_while(|(expiry, _)| *expiry <= coarsetime::Clock::now_since_epoch().as_secs())
            .map(|(_, topic)| topic.clone())
            .collect();

        if !expired_topics.is_empty() {
            for topic in expired_topics {
                self.remove(&topic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RetainedMessage, RetainedTrie};
    use crate::mqtt::QoS;

    fn msg(message: &'static str) -> RetainedMessage {
        RetainedMessage {
            qos: QoS::AtMostOnce,
            topic: "a/b/c".to_string(),
            payload: message.into(),
            properties: vec![],
            expiry_at: None,
        }
    }

    #[test]
    fn test_find_matches() {
        let mut trie = RetainedTrie::new();
        trie.insert("a/b/c", msg("msg1"));
        trie.insert("a/foo/c", msg("msg2"));
        trie.insert("a/foo/d", msg("msg3"));
        trie.insert("x/y/z", msg("msg4"));

        // Test '+'
        let matches = trie.find_matches_for_filter("a/+/c");
        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches[0].payload.windows(4).any(|w| w == b"msg1")
                || matches[0].payload.windows(4).any(|w| w == b"msg2"),
            true
        );

        // Test '#'
        let matches = trie.find_matches_for_filter("a/#");
        assert_eq!(matches.len(), 3);

        // Test no match
        let matches = trie.find_matches_for_filter("a/+/z");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_remove_and_prune() {
        let mut trie = RetainedTrie::new();
        trie.insert("a/b/c", msg("msg1"));
        trie.insert("a/b/d", msg("msg2"));

        // Remove one leaf
        trie.remove("a/b/c");
        let matches = trie.find_matches_for_filter("a/b/c");
        assert!(matches.is_empty());
        // The other should still exist
        let matches = trie.find_matches_for_filter("a/b/d");
        assert_eq!(matches.len(), 1);

        // Remove the other leaf, which should prune the 'b' and 'a' nodes
        trie.remove("a/b/d");
        let matches = trie.find_matches_for_filter("a/#");
        assert!(matches.is_empty());
    }
}
