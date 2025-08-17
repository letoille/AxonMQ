use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use tokio::sync::mpsc;
use tracing::{debug, trace};

use crate::mqtt::QoS;
use crate::mqtt::command::ClientCommand;
use crate::mqtt::helper::BrokerHelper;
use crate::mqtt::protocol::property::Property;
use crate::operator::retain_trie;
use crate::utils as g_utils;

use super::client::ClientInfo;
use super::command::{OperatorCommand, OutSender};
use super::helper::Helper;
use super::out::mqtt::MqttOutSender;
use super::retain_trie::RetainedTrie;
use super::trie::TopicTrie;
use super::utils;

static NEXT_INDEX: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct Router<T: OutSender + Default> {
    command_rx: Option<mpsc::Receiver<OperatorCommand<T>>>,
    command_tx: mpsc::Sender<OperatorCommand<T>>,

    trie: Option<TopicTrie<ClientInfo<T>>>,
    retain_trie: Option<RetainedTrie>,
}

impl Router<MqttOutSender> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1024);
        Router {
            command_rx: Some(rx),
            command_tx: tx,
            trie: Some(TopicTrie::new()),
            retain_trie: Some(RetainedTrie::new()),
        }
    }

    pub fn helper(&self) -> Helper<MqttOutSender> {
        Helper {
            operator_tx: self.command_tx.clone(),
        }
    }

    fn find_clients<'a>(
        cache: &'a mut HashMap<String, Vec<ClientInfo<MqttOutSender>>>,
        trie: &'a TopicTrie<ClientInfo<MqttOutSender>>,
        client_id: &str,
        topic: &str,
    ) -> (
        Vec<&'a mut ClientInfo<MqttOutSender>>,
        HashMap<String, Vec<&'a mut ClientInfo<MqttOutSender>>>,
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
            let mut group_clients_map: HashMap<String, Vec<&mut ClientInfo<MqttOutSender>>> =
                HashMap::new();
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

    fn send_msg(
        broker_helper: BrokerHelper,
        cache: &mut HashMap<String, Vec<ClientInfo<MqttOutSender>>>,
        trie: &TopicTrie<ClientInfo<MqttOutSender>>,
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    ) {
        let (clients_iters, group_clients_map) =
            Self::find_clients(cache, trie, &client_id, &topic);
        let clients_iters = &mut clients_iters.into_iter().peekable();

        let client_send_msg = |client: &mut ClientInfo<MqttOutSender>, msg: ClientCommand| {
            if client.store && expiry_at.is_some() && expiry_at.unwrap() > 0 {
                if client.sender.send(msg.clone()).is_err() {
                    broker_helper
                        .store_msg(&client.client_id, msg)
                        .unwrap_or(());
                }
            } else {
                let _ = client.sender.send(msg);
            }
        };

        for (_group, mut clients) in group_clients_map.into_iter() {
            let current_index = NEXT_INDEX.fetch_add(1, Ordering::Relaxed);
            let index = current_index % clients.len();
            let client = &mut clients[index];

            client_send_msg(
                client,
                ClientCommand::Publish {
                    qos: qos.min(client.qos),
                    retain,
                    topic: topic.clone(),
                    payload: payload.clone(),
                    properties: properties.clone(),
                    expiry_at,
                },
            );
        }

        while let Some(client) = clients_iters.next() {
            trace!(
                "send to client: {}, topic: {}, expiry_at: {:?}",
                g_utils::TruncateDisplay::new(&client.client_id, 24),
                g_utils::TruncateDisplay::new(&topic, 128),
                expiry_at
            );
            if clients_iters.peek().is_some() {
                let msg = ClientCommand::Publish {
                    qos: qos.min(client.qos),
                    retain,
                    topic: topic.clone(),
                    payload: payload.clone(),
                    properties: properties.clone(),
                    expiry_at,
                };
                client_send_msg(client, msg);
            } else {
                let msg = ClientCommand::Publish {
                    qos: qos.min(client.qos),
                    retain,
                    topic,
                    payload,
                    properties,
                    expiry_at,
                };
                client_send_msg(client, msg);
                break;
            }
        }
    }

    pub fn run(&mut self, broker_helper: BrokerHelper) {
        let mut command_rx = self.command_rx.take().unwrap();
        let mut trie = self.trie.take().unwrap();
        let mut cache: HashMap<String, Vec<ClientInfo<MqttOutSender>>> = HashMap::new();
        let mut retain_trie = self.retain_trie.take().unwrap();

        use OperatorCommand::*;

        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    MqttSubscribe {
                        client_id,
                        share_group,
                        topic,
                        qos,
                        no_local,
                        store,
                        sender,
                    } => {
                        debug!(
                            "insert client: {}, topic: {}",
                            g_utils::TruncateDisplay::new(&client_id, 24),
                            g_utils::TruncateDisplay::new(&topic, 128)
                        );

                        let msgs = retain_trie.find_matches_for_filter(&topic);
                        for msg in msgs {
                            if let Some(expiry_at) = msg.expiry_at {
                                if expiry_at <= coarsetime::Instant::now().elapsed().as_secs() {
                                    continue;
                                }
                            }
                            let cmd = ClientCommand::Publish {
                                qos: msg.qos,
                                retain: true,
                                topic: msg.topic.clone(),
                                payload: msg.payload.clone(),
                                properties: msg.properties.clone(),
                                expiry_at: msg.expiry_at,
                            };
                            if sender.send(cmd.clone()).is_err()
                                && msg.expiry_at.is_some()
                                && msg.expiry_at.unwrap() > 0
                            {
                                let _ = broker_helper.store_msg(&client_id, cmd);
                            }
                        }
                        cache.retain(|k, _| !utils::topic_match(&topic, k));
                        trie.insert(
                            &topic,
                            ClientInfo::new(
                                client_id,
                                share_group,
                                topic.clone(),
                                no_local,
                                sender,
                                qos,
                                store,
                            ),
                        );
                    }
                    MqttUnsubscribe {
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
                        trie.remove(&topic, &ClientInfo::default(client_id, share_group));
                    }
                    MqttRemoveClient { client_id } => {
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
                    MqttPublish {
                        client_id,
                        retain,
                        qos,
                        topic,
                        payload,
                        properties,
                        expiry_at,
                    } => {
                        if retain {
                            if payload.is_empty() {
                                retain_trie.remove(&topic);
                            } else {
                                retain_trie.insert(
                                    &topic,
                                    retain_trie::RetainedMessage {
                                        qos: QoS::AtMostOnce,
                                        topic: topic.clone(),
                                        payload: payload.clone(),
                                        properties: properties.clone(),
                                        expiry_at,
                                    },
                                );
                            }
                        }

                        Self::send_msg(
                            broker_helper.clone(),
                            &mut cache,
                            &trie,
                            client_id,
                            retain,
                            qos,
                            topic,
                            payload,
                            properties,
                            expiry_at,
                        );
                    }
                    MqttPurgeExpiry => {
                        retain_trie.purge_expired();
                    }
                }
            }
        });
    }
}
