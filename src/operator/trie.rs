use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone)]
struct TrieNode<T> {
    // +
    pub single_wildcard_child: Option<Box<TrieNode<T>>>,
    // #
    pub multi_wildcard_matches: Vec<T>,

    // a, b, sensors
    pub literal_children: HashMap<String, TrieNode<T>>,
    pub exact_matches: Vec<T>,
}

impl<T> TrieNode<T> {
    fn is_empty(&self) -> bool {
        self.exact_matches.is_empty()
            && self.multi_wildcard_matches.is_empty()
            && self.literal_children.is_empty()
            && self.single_wildcard_child.is_none()
    }
}

pub trait ClientId {
    fn client_id(&self) -> &str;
}

#[derive(Debug)]
pub struct TopicTrie<T> {
    root: TrieNode<T>,
}

impl<T> TopicTrie<T>
where
    T: Clone + Eq + Hash + PartialEq + ClientId,
{
    pub fn new() -> Self {
        TopicTrie {
            root: TrieNode {
                single_wildcard_child: None,
                multi_wildcard_matches: Vec::new(),
                literal_children: HashMap::new(),
                exact_matches: Vec::new(),
            },
        }
    }

    fn default() -> Box<TrieNode<T>> {
        Box::new(TrieNode {
            single_wildcard_child: None,
            multi_wildcard_matches: Vec::new(),
            literal_children: HashMap::new(),
            exact_matches: Vec::new(),
        })
    }

    pub fn insert(&mut self, topic: &str, value: T) {
        let mut current_node = &mut self.root;
        let parts: Vec<&str> = topic.split('/').collect();
        let last_index = parts.len().saturating_sub(1);

        for (i, part) in parts.iter().enumerate() {
            let part_string = part.to_string();

            if part_string == "#" {
                if i == last_index {
                    current_node.multi_wildcard_matches.push(value.clone());
                }
                return;
            } else if part_string == "+" {
                current_node = current_node
                    .single_wildcard_child
                    .get_or_insert_with(Self::default);
            } else {
                current_node = current_node
                    .literal_children
                    .entry(part_string)
                    .or_insert_with(|| TrieNode {
                        single_wildcard_child: None,
                        multi_wildcard_matches: Vec::new(),
                        literal_children: HashMap::new(),
                        exact_matches: Vec::new(),
                    });
            }
        }

        current_node.exact_matches.push(value);
    }

    pub fn find_matches(&self, topic: &str) -> Vec<&T> {
        let mut results = HashSet::new();
        let topic_parts: Vec<&str> = topic.split('/').collect();
        self.recursive_match(&self.root, &topic_parts, &mut results);
        results.into_iter().collect()
    }

    fn recursive_match<'a>(
        &self,
        current_node: &'a TrieNode<T>,
        topic_parts: &[&str],
        results: &mut HashSet<&'a T>,
    ) {
        for subscriber in &current_node.multi_wildcard_matches {
            results.insert(subscriber);
        }

        if topic_parts.is_empty() {
            for subscriber in &current_node.exact_matches {
                results.insert(subscriber);
            }
            return;
        }

        let current_level = topic_parts[0];
        let remaining_parts = &topic_parts[1..];

        if let Some(wildcard_child) = &current_node.single_wildcard_child {
            self.recursive_match(wildcard_child, remaining_parts, results);
        }

        if let Some(literal_child) = current_node.literal_children.get(current_level) {
            self.recursive_match(literal_child, remaining_parts, results);
        }
    }

    pub fn remove(&mut self, topic: &str, value: &T) {
        let parts: Vec<&str> = topic.split('/').collect();
        Self::recursive_remove(&mut self.root, &parts, value);
    }

    fn recursive_remove(node: &mut TrieNode<T>, topic_parts: &[&str], value: &T) -> bool {
        if let Some((current_part, remaining_parts)) = topic_parts.split_first() {
            if *current_part == "#" {
                node.multi_wildcard_matches.retain(|v| v != value);
            } else if *current_part == "+" {
                if let Some(mut child) = node.single_wildcard_child.take() {
                    if !Self::recursive_remove(&mut child, remaining_parts, value) {
                        node.single_wildcard_child = Some(child);
                    }
                }
            } else {
                let part_string = current_part.to_string();
                if let Some(mut child) = node.literal_children.remove(&part_string) {
                    if !Self::recursive_remove(&mut child, remaining_parts, value) {
                        node.literal_children.insert(part_string, child);
                    }
                }
            }
        } else {
            node.exact_matches.retain(|v| v != value);
        }

        node.is_empty()
    }

    pub fn remove_client(&mut self, client_id: &str) {
        Self::recursive_remove_client(&mut self.root, client_id);
    }

    fn recursive_remove_client(node: &mut TrieNode<T>, client_id: &str) -> bool {
        node.exact_matches.retain(|v| v.client_id() != client_id);
        node.multi_wildcard_matches
            .retain(|v| v.client_id() != client_id);

        node.literal_children
            .retain(|_key, child_node| !Self::recursive_remove_client(child_node, client_id));

        if let Some(mut child) = node.single_wildcard_child.take() {
            if !Self::recursive_remove_client(&mut child, client_id) {
                node.single_wildcard_child = Some(child);
            }
        }

        node.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::TopicTrie;

    #[test]
    fn test_insert_and_match() {
        let mut trie = TopicTrie::new();
        trie.insert("a/b/c", "client1");
        trie.insert("a/+/c", "client2");
        trie.insert("a/b/#", "client3");
        trie.insert("#", "client4");

        let mut matches = trie.find_matches("a/b/c");
        matches.sort();
        assert_eq!(
            matches,
            vec![&"client1", &"client2", &"client3", &"client4"]
        );

        let mut matches = trie.find_matches("a/foo/c");
        matches.sort();
        assert_eq!(matches, vec![&"client2", &"client4"]);

        let mut matches = trie.find_matches("a/b/d");
        matches.sort();
        assert_eq!(matches, vec![&"client3", &"client4"]);
    }

    #[test]
    fn test_remove_exact() {
        let mut trie = TopicTrie::new();
        trie.insert("a/b/c", "client1");
        trie.insert("a/b/c", "client2");

        let matches = trie.find_matches("a/b/c");
        assert_eq!(matches.len(), 2);

        trie.remove("a/b/c", &"client1");
        let mut matches = trie.find_matches("a/b/c");
        matches.sort();
        assert_eq!(matches, vec![&"client2"]);
    }

    #[test]
    fn test_remove_wildcard() {
        let mut trie = TopicTrie::new();
        trie.insert("a/#", "client1");
        trie.insert("a/b", "client2");

        let mut matches = trie.find_matches("a/b");
        matches.sort();
        assert_eq!(matches, vec![&"client1", &"client2"]);

        trie.remove("a/#", &"client1");
        let mut matches = trie.find_matches("a/b");
        matches.sort();
        assert_eq!(matches, vec![&"client2"]);
    }

    #[test]
    fn test_remove_and_prune() {
        let mut trie = TopicTrie::<i32>::new();
        trie.insert("a/b/c", 1);

        assert!(!trie.find_matches("a/b/c").is_empty());

        trie.remove("a/b/c", &1);

        assert!(trie.find_matches("a/b/c").is_empty());
    }
}
