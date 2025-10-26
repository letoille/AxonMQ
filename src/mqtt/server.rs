use std::collections::HashMap;

use tokio::{sync::mpsc, task, time};
use tracing::debug;

use crate::operator::sink::local::LocalClientSink;
use crate::{
    CONFIG,
    mqtt::helper::ClientHelper,
    operator::{Operator, helper::Helper as OperatorHelper},
    utils as g_utils,
};

use super::{
    MqttProtocolVersion,
    code::ReturnCode,
    command::{BrokerAck, BrokerCommand, ClientCommand},
    helper::BrokerHelper,
    protocol::{
        conn::ConnAck,
        property::Property,
        subscribe::{SubAck, SubscribeOption, UnsubAck},
        will::Will,
    },
    retain_trie::{RetainedMessage, RetainedTrie},
    utils,
};

pub struct Client {
    client_id: String,
    version: MqttProtocolVersion,

    connected: bool,
    disconnected_tm: u64,

    clear_start: bool,
    expiry: u32,

    subscribes: HashMap<String, (SubscribeOption, Vec<Property>)>,
    client_helper: ClientHelper,

    will: Option<Will>,
}

pub struct Broker {
    operator: Option<Operator>,

    broker_tx: mpsc::Sender<BrokerCommand>,
    broker_rx: Option<mpsc::Receiver<BrokerCommand>>,

    store_clients: Option<HashMap<String, Client>>,
    clean_clients: Option<HashMap<String, Client>>,

    retain_trie: Option<RetainedTrie>,
}

impl Broker {
    pub async fn new() -> Self {
        let (broker_tx, broker_rx) = mpsc::channel::<BrokerCommand>(128);
        let operator = Operator::new().await;

        Broker {
            operator: Some(operator),
            broker_tx,
            broker_rx: Some(broker_rx),
            store_clients: Some(HashMap::new()),
            clean_clients: Some(HashMap::new()),
            retain_trie: Some(RetainedTrie::new()),
        }
    }

    pub fn operator_helper(&self) -> OperatorHelper {
        self.operator.as_ref().unwrap().matcher_helper()
    }

    pub fn get_helper(&self) -> BrokerHelper {
        BrokerHelper {
            broker_tx: self.broker_tx.clone(),
        }
    }

    fn prepare_will_message(broker_helper: BrokerHelper, client_id: String, will: Will) {
        task::spawn(async move {
            if will.will_delay_interval > 0 {
                time::sleep(time::Duration::from_secs(will.will_delay_interval as u64)).await;
            }
            broker_helper
                .will_publish(
                    client_id.as_str(),
                    will.topic,
                    will.payload,
                    will.qos,
                    will.retain,
                    Property::filter_for_publish(will.properties),
                    will.expiry_interval
                        .map(|v| coarsetime::Clock::now_since_epoch().as_secs() + v as u64),
                )
                .await
                .ok();
        });
    }

    async fn handle_message(
        store_clients: &mut HashMap<String, Client>,
        clean_clients: &mut HashMap<String, Client>,
        cmd: BrokerCommand,
        operator_helper: &mut OperatorHelper,
        broker_helper: BrokerHelper,
        store_msgs: &mut HashMap<String, Vec<ClientCommand>>,
        retain_trie: &mut RetainedTrie,
    ) {
        use BrokerCommand::*;
        match cmd {
            Connect {
                connect,
                resp,
                client_tx,
            } => {
                let old_client = clean_clients
                    .remove(&connect.client_id)
                    .or_else(|| store_clients.remove(&connect.client_id));

                if let Some(ref old_client) = old_client {
                    if old_client.connected {
                        debug!("client connected, disconnect old session",);
                        old_client
                            .client_helper
                            .disconnect(ReturnCode::SessionTakenOver)
                            .await
                            .ok();
                        if let Some(will) = &old_client.will {
                            Self::prepare_will_message(
                                broker_helper.clone(),
                                old_client.client_id.clone(),
                                will.clone(),
                            );
                        }
                        let _ = operator_helper
                            .remove_client(old_client.client_id.clone())
                            .await;
                    }
                }

                let client = if connect.clean_start {
                    Client {
                        client_id: connect.client_id.clone(),
                        version: connect.version,
                        connected: true,
                        disconnected_tm: 0,
                        clear_start: connect.clean_start,
                        expiry: connect.session_expiry_interval.unwrap_or(0),
                        client_helper: ClientHelper::new(client_tx),
                        subscribes: HashMap::new(),
                        will: connect.will,
                    }
                } else {
                    Client {
                        client_id: connect.client_id.clone(),
                        version: connect.version,
                        connected: true,
                        disconnected_tm: 0,
                        clear_start: connect.clean_start,
                        expiry: connect.session_expiry_interval.unwrap_or(0),
                        client_helper: ClientHelper::new(client_tx),
                        subscribes: old_client
                            .as_ref()
                            .map(|c| c.subscribes.clone())
                            .unwrap_or_default(),
                        will: connect.will,
                    }
                };

                resp.send(BrokerAck::ConnAck(ConnAck::new(
                    client.expiry > 0,
                    ReturnCode::Success,
                    if connect.generate_client_id {
                        Some(client.client_id.clone())
                    } else {
                        None
                    },
                )))
                .ok();
                debug!(
                    "accept connected: {} [version: {}, clean: {}, expiry: {}]",
                    g_utils::TruncateDisplay::new(&client.client_id, 24),
                    client.version,
                    client.clear_start,
                    client.expiry
                );
                operator_helper
                    .remove_client(client.client_id.clone())
                    .await
                    .ok();
                if client.clear_start {
                    store_msgs.remove(&connect.client_id);
                } else {
                    for (topic, (options, _)) in &client.subscribes {
                        let (group, actual_topic) = if utils::is_shared_subscription(topic) {
                            utils::parse_shared_subscription(topic).unwrap_or(("", topic.as_str()))
                        } else {
                            ("", topic.as_str())
                        };
                        let _ = operator_helper
                            .subscribe(
                                client.client_id.clone(),
                                if group.is_empty() {
                                    None
                                } else {
                                    Some(group.to_string())
                                },
                                actual_topic.to_string(),
                                options.qos,
                                options.no_local,
                                client.expiry > 0,
                                LocalClientSink::new(
                                    client.client_helper.get(),
                                    broker_helper.clone(),
                                ),
                            )
                            .await;
                    }

                    if let Some(msgs) = store_msgs.remove(&connect.client_id) {
                        for msg in msgs {
                            let _ = client.client_helper.send(msg);
                        }
                    }
                }
                if client.expiry > 0 {
                    store_clients.insert(connect.client_id.clone(), client);
                } else {
                    clean_clients.insert(connect.client_id.clone(), client);
                }
            }
            Subscribe {
                client_id,
                subscribe,
                resp,
            } => {
                if let Some(client) = store_clients
                    .get_mut(&client_id)
                    .or_else(|| clean_clients.get_mut(&client_id))
                {
                    let mut codes = vec![];
                    for (topic, options) in subscribe.topics {
                        if utils::sub_topic_valid(&topic)
                            && (utils::parse_shared_subscription(&topic).is_ok()
                                || !utils::is_shared_subscription(&topic))
                        {
                            let (group, actual_topic) =
                                utils::parse_shared_subscription(&topic).unwrap_or(("", &topic));
                            if !client.subscribes.contains_key(&topic) {
                                let msgs = retain_trie.find_matches_for_filter(&topic);

                                let _ = operator_helper
                                    .subscribe(
                                        client_id.clone(),
                                        if group.is_empty() {
                                            None
                                        } else {
                                            Some(group.to_string())
                                        },
                                        actual_topic.to_string(),
                                        options.qos,
                                        options.no_local,
                                        client.expiry > 0,
                                        LocalClientSink::new(
                                            client.client_helper.get(),
                                            broker_helper.clone(),
                                        ),
                                    )
                                    .await;

                                for msg in msgs {
                                    let _ = operator_helper
                                        .publish(
                                            client_id.clone(),
                                            true,
                                            msg.qos,
                                            msg.topic.clone(),
                                            msg.payload.clone(),
                                            msg.properties.clone(),
                                            msg.expiry_at,
                                        )
                                        .await;
                                }
                                client
                                    .subscribes
                                    .insert(topic, (options, subscribe.properties.clone()));
                            }

                            codes.push(ReturnCode::Success);
                        } else {
                            codes.push(ReturnCode::TopicFilterInvalid);
                        }
                    }
                    let ack = SubAck::new(subscribe.packet_id, codes);
                    resp.send(BrokerAck::SubAck(ack)).ok();
                } else {
                    let ack = SubAck::new(
                        subscribe.packet_id,
                        vec![ReturnCode::UnspecifiedError; subscribe.topics.len()],
                    );
                    resp.send(BrokerAck::SubAck(ack)).ok();
                }
            }
            Unsubscribe {
                client_id,
                unsubscribe,
                resp,
            } => {
                if let Some(client) = store_clients
                    .get_mut(&client_id)
                    .or_else(|| clean_clients.get_mut(&client_id))
                {
                    let len = unsubscribe.topics.len();
                    for topic in unsubscribe.topics.into_iter() {
                        if client.subscribes.remove(&topic).is_some() {
                            let (share_group, actual_topic) =
                                if utils::is_shared_subscription(&topic) {
                                    utils::parse_shared_subscription(&topic).unwrap_or(("", &topic))
                                } else {
                                    ("", topic.as_str())
                                };
                            let _ = operator_helper
                                .unsubscribe(
                                    client_id.clone(),
                                    if share_group.is_empty() {
                                        None
                                    } else {
                                        Some(share_group.to_string())
                                    },
                                    actual_topic.to_string(),
                                )
                                .await;
                        }
                    }
                    let ack = UnsubAck::new(unsubscribe.packet_id, vec![ReturnCode::Success; len]);
                    resp.send(BrokerAck::UnsubAck(ack)).ok();
                } else {
                    let ack = UnsubAck::new(
                        unsubscribe.packet_id,
                        vec![ReturnCode::UnspecifiedError; unsubscribe.topics.len()],
                    );
                    resp.send(BrokerAck::UnsubAck(ack)).ok();
                }
            }
            Disconnected(client_id, code) => {
                if let Some(client) = clean_clients.remove(&client_id) {
                    let _ = operator_helper
                        .remove_client(client.client_id.clone())
                        .await;
                    if let Some(ref will) = client.will {
                        if code != ReturnCode::Success {
                            Self::prepare_will_message(
                                broker_helper,
                                client_id.clone(),
                                will.clone(),
                            );
                        }
                    }
                } else {
                    if let Some(client) = store_clients.get_mut(&client_id) {
                        client.disconnected_tm = time::Instant::now().elapsed().as_secs();
                        client.connected = false;

                        if let Some(ref will) = client.will {
                            if code != ReturnCode::Success {
                                Self::prepare_will_message(
                                    broker_helper,
                                    client_id.clone(),
                                    will.clone(),
                                );
                            }
                        }
                    }
                }
            }
            WillPublish {
                client_id,
                retain,
                qos,
                topic,
                payload,
                properties,
                expiry_at,
            } => {
                let client = store_clients
                    .get(&client_id)
                    .or_else(|| clean_clients.get(&client_id));
                if client.is_none() || !client.unwrap().connected {
                    if retain {
                        broker_helper
                            .retain_message(
                                topic.clone(),
                                qos,
                                payload.clone(),
                                properties.clone(),
                                expiry_at,
                            )
                            .await
                            .ok();
                    }

                    let _ = operator_helper
                        .publish(
                            client_id, retain, qos, topic, payload, properties, expiry_at,
                        )
                        .await;
                }
            }
            StoreMsg { client_id, msg } => {
                if let Some(client) = store_clients.get(&client_id) {
                    if client.connected {
                        let _ = client.client_helper.send(msg);
                    } else if client.expiry > 0 {
                        let msgs = store_msgs.entry(client_id).or_insert_with(Vec::new);
                        if msgs.len()
                            >= CONFIG
                                .get()
                                .unwrap()
                                .mqtt
                                .settings
                                .max_store_msgs_per_client
                        {
                            msgs.remove(0);
                        }
                        msgs.push(msg);
                    }
                }
            }
            RetainMessage {
                topic,
                qos,
                payload,
                properties,
                expiry_at,
            } => {
                if payload.is_empty() {
                    retain_trie.remove(&topic);
                } else {
                    retain_trie.insert(
                        &topic,
                        RetainedMessage {
                            qos,
                            topic: topic.clone(),
                            payload: payload.clone(),
                            properties: properties.clone(),
                            expiry_at,
                        },
                    );
                }
            }
        }
    }

    pub async fn run(&mut self) {
        let mut operator = self.operator.take().unwrap();
        operator.run();

        let mut broker_rx = self.broker_rx.take().unwrap();
        let mut store_clients = self.store_clients.take().unwrap();
        let mut clean_clients = self.clean_clients.take().unwrap();

        let mut clean_tk = time::interval(time::Duration::from_secs(60));
        let mut operator_helper = operator.matcher_helper();
        let broker_helper = self.get_helper();
        let mut store_msgs: HashMap<String, Vec<ClientCommand>> = HashMap::new();
        let mut retain_trie = self.retain_trie.take().unwrap();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(cmd) = broker_rx.recv() => {
                        Self::handle_message(&mut store_clients, &mut clean_clients, cmd, &mut operator_helper, broker_helper.clone(), &mut store_msgs, &mut retain_trie).await;
                    }
                    _ = clean_tk.tick() => {
                        let mut remove_ids = Vec::new();
                        store_clients.retain(|_, client| {
                            if client.connected {
                                true
                            } else if client.expiry == 0 {
                                remove_ids.push(client.client_id.clone());
                                false
                            } else {
                                let now = time::Instant::now().elapsed().as_secs();
                                let result = now - client.disconnected_tm < client.expiry as u64;
                                if !result {
                                    remove_ids.push(client.client_id.clone());
                                }
                                result
                            }
                        });
                        for client_id in remove_ids {
                            store_msgs.remove(&client_id);
                            let _ = operator_helper.remove_client(client_id).await;
                        }
                        retain_trie.purge_expired();
                    }
                }
            }
        });
    }
}
