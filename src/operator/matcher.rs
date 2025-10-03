use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::mpsc;
use tracing::{debug, trace};

use crate::message::Message;
use crate::mqtt::QoS;
use crate::utils as g_utils;

use super::command::OperatorCommand;
use super::helper::Helper;
use super::sink::{DefaultSink, Sink};
use super::trie::{ClientId, TopicTrie};
use super::utils;

static NEXT_INDEX: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct Subscriber {
    client_id: String,

    share_group: Option<String>,
    topic: String,

    qos: QoS,
    no_local: bool,
    persist: bool,

    sink: Box<dyn Sink>,
}

impl Subscriber {
    pub fn default(client_id: String, share_group: Option<String>) -> Self {
        Subscriber {
            client_id,
            share_group,
            topic: String::new(),
            no_local: false,
            sink: DefaultSink::new(),
            qos: QoS::AtMostOnce,
            persist: false,
        }
    }

    pub fn filter(&self, client_id: &str) -> bool {
        !(self.no_local && self.client_id == client_id)
    }
}

impl PartialEq for Subscriber {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.share_group == other.share_group
    }
}

impl Eq for Subscriber {}

impl Hash for Subscriber {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.client_id.hash(state);
    }
}

impl ClientId for Subscriber {
    fn client_id(&self) -> &str {
        &self.client_id
    }
}

pub(crate) struct Matcher {
    command_rx: Option<mpsc::Receiver<OperatorCommand>>,
    command_tx: mpsc::Sender<OperatorCommand>,

    trie: Option<TopicTrie<Subscriber>>,
}

impl Matcher {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1024);
        Matcher {
            command_rx: Some(rx),
            command_tx: tx,
            trie: Some(TopicTrie::new()),
        }
    }

    pub fn helper(&self) -> Helper {
        Helper::new(self.command_tx.clone())
    }

    pub fn run(&mut self) {
        let mut command_rx = self.command_rx.take().unwrap();
        let mut trie = self.trie.take().unwrap();
        let mut cache: HashMap<String, Vec<Subscriber>> = HashMap::new();

        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                Self::process_command(&mut trie, &mut cache, cmd);
            }
        });
    }

    fn process_command(
        trie: &mut TopicTrie<Subscriber>,
        cache: &mut HashMap<String, Vec<Subscriber>>,
        cmd: OperatorCommand,
    ) {
        use OperatorCommand::*;
        match cmd {
            Subscribe {
                client_id,
                share_group,
                topic,
                qos,
                no_local,
                persist,
                sink,
            } => {
                debug!(
                    "insert client: {}, topic: {}",
                    g_utils::TruncateDisplay::new(&client_id, 24),
                    g_utils::TruncateDisplay::new(&topic, 128)
                );
                cache.retain(|k, _| !utils::topic_match(&topic, k));
                trie.insert(
                    &topic,
                    Subscriber {
                        client_id,
                        share_group,
                        topic: topic.clone(),
                        qos,
                        no_local,
                        persist,
                        sink,
                    },
                );
            }
            Unsubscribe {
                client_id,
                share_group,
                topic,
            } => {
                debug!(
                    "remove client: {}, topic: {}",
                    g_utils::TruncateDisplay::new(&client_id, 24),
                    g_utils::TruncateDisplay::new(&topic, 128)
                );
                cache.retain(|_k, v| {
                    v.retain(|info| {
                        !(info.client_id == client_id
                            && info.topic == topic
                            && info.share_group == share_group)
                    });
                    !v.is_empty()
                });
                trie.remove(&topic, &&Subscriber::default(client_id, share_group));
            }
            Publish {
                client_id,
                retain,
                qos,
                topic,
                payload,
                properties,
                expiry_at,
            } => {
                let (clients_iters, group_clients_map) =
                    Self::find_clients(cache, trie, &client_id, &topic);
                let clients_iters = &mut clients_iters.into_iter();

                for (_group, mut clients) in group_clients_map.into_iter() {
                    let current_index = NEXT_INDEX.fetch_add(1, Ordering::Relaxed);
                    let index = current_index % clients.len();
                    let client = &mut clients[index];

                    let _ = client.sink.deliver(
                        Message::new(
                            client.client_id.clone(),
                            topic.clone(),
                            qos.min(client.qos),
                            retain,
                            expiry_at,
                            payload.clone(),
                            properties.clone(),
                        ),
                        client.persist,
                    );
                }

                while let Some(client) = clients_iters.next() {
                    trace!(
                        "send to client: {}, topic: {}, expiry_at: {:?}",
                        g_utils::TruncateDisplay::new(&client.client_id, 24),
                        g_utils::TruncateDisplay::new(&topic, 128),
                        expiry_at
                    );
                    let _ = client.sink.deliver(
                        Message::new(
                            client.client_id.clone(),
                            topic.clone(),
                            qos.min(client.qos),
                            retain,
                            expiry_at,
                            payload.clone(),
                            properties.clone(),
                        ),
                        client.persist,
                    );
                }
            }
            RemoveClient { client_id } => {
                debug!(
                    "remove client: {}",
                    g_utils::TruncateDisplay::new(&client_id, 24)
                );
                trie.remove_client(&client_id);
                cache.retain(|_k, v| {
                    v.retain(|info| info.client_id != client_id);
                    !v.is_empty()
                });
            }
        }
    }

    fn find_clients<'a>(
        cache: &'a mut HashMap<String, Vec<Subscriber>>,
        trie: &'a TopicTrie<Subscriber>,
        client_id: &str,
        topic: &str,
    ) -> (
        Vec<&'a mut Subscriber>,
        HashMap<String, Vec<&'a mut Subscriber>>,
    ) {
        if !cache.contains_key(topic) {
            let clients = trie.find_matches(topic);
            if clients.len() > 0 {
                let clients = clients.into_iter().cloned().collect::<Vec<_>>();
                cache.insert(topic.to_string(), clients);
            }
        }

        let clients = cache.get_mut(topic);
        if let Some(clients) = clients {
            let clients = clients.iter_mut().filter(|c| c.filter(client_id));
            let (clients_iters, shared_clients): (Vec<_>, Vec<_>) =
                clients.partition(|c| c.share_group.is_none());
            let mut group_clients_map: HashMap<String, Vec<&mut Subscriber>> = HashMap::new();
            for client in shared_clients.into_iter() {
                if let Some(group) = &client.share_group {
                    group_clients_map
                        .entry(group.clone())
                        .or_insert_with(Vec::new)
                        .push(client);
                }
            }
            (clients_iters, group_clients_map)
        } else {
            (vec![], HashMap::new())
        }
    }
}
