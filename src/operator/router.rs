use std::collections::HashMap;

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
        if let Some(clients) = cache.get(&topic) {
            let mut clients_iters = clients.iter().filter(|c| c.filter(&client_id)).peekable();
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
                    if client.store && expiry_at.is_some() && expiry_at.unwrap() > 0 {
                        if client.sender.send(msg.clone()).is_err() {
                            broker_helper
                                .store_msg(&client.client_id, msg)
                                .unwrap_or(());
                        }
                    } else {
                        let _ = client.sender.send(msg);
                    }
                } else {
                    let msg = ClientCommand::Publish {
                        qos: qos.min(client.qos),
                        retain,
                        topic,
                        payload,
                        properties,
                        expiry_at,
                    };
                    if client.store && expiry_at.is_some() && expiry_at.unwrap() > 0 {
                        if client.sender.send(msg.clone()).is_err() {
                            broker_helper
                                .store_msg(&client.client_id, msg)
                                .unwrap_or(());
                        }
                    } else {
                        let _ = client.sender.send(msg);
                    }
                    break;
                }
            }
        } else {
            let clients = trie.find_matches(&topic);
            let mut clients_iters = clients.iter().filter(|c| c.filter(&client_id)).peekable();
            while let Some(client) = clients_iters.next() {
                trace!(
                    "send to client: {}, topic: {:.128}, expiry_at: {:?}",
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
                        properties: vec![],
                        expiry_at,
                    };
                    if client.store && expiry_at.is_some() && expiry_at.unwrap() > 0 {
                        if client.sender.send(msg.clone()).is_err() {
                            broker_helper
                                .store_msg(&client.client_id, msg)
                                .unwrap_or(());
                        }
                    } else {
                        let _ = client.sender.send(msg);
                    }
                } else {
                    let msg = ClientCommand::Publish {
                        qos: qos.min(client.qos),
                        retain,
                        topic: topic.clone(),
                        payload,
                        properties: vec![],
                        expiry_at,
                    };
                    if client.store && expiry_at.is_some() && expiry_at.unwrap() > 0 {
                        if client.sender.send(msg.clone()).is_err() {
                            broker_helper
                                .store_msg(&client.client_id, msg)
                                .unwrap_or(());
                        }
                    } else {
                        let _ = client.sender.send(msg);
                    }
                    break;
                }
            }
            if clients.len() > 0 {
                cache.insert(topic.clone(), clients.into_iter().cloned().collect());
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
                            ClientInfo::new(client_id, topic.clone(), no_local, sender, qos, store),
                        );
                    }
                    MqttUnsubscribe { client_id, topic } => {
                        debug!(
                            "remove client: {}, topic: {}",
                            g_utils::TruncateDisplay::new(&client_id, 24),
                            g_utils::TruncateDisplay::new(&topic, 128)
                        );
                        cache.retain(|_k, v| {
                            v.retain(|info| !(info.client_id == client_id && info.topic == topic));
                            !v.is_empty()
                        });
                        trie.remove(&topic, &ClientInfo::default(client_id));
                    }
                    MqttRemoveClient { client_id } => {
                        debug!(
                            "remove client: {}",
                            g_utils::TruncateDisplay::new(&client_id, 24)
                        );
                        trie.remove_client(&ClientInfo::default(client_id.clone()));
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
