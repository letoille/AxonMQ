use std::collections::HashMap;
use std::net::SocketAddr;

use coarsetime;
use futures_util::{SinkExt, stream::StreamExt as _};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::{sync::mpsc, time};
use tokio_util::codec::Framed;
use tracing::{Instrument, debug, info, warn};

use crate::CONFIG;
use crate::operator::helper::Helper as OperatorHelper;
use crate::utils as g_utils;

use crate::mqtt::protocol::{codec::MessageCodec, conn::Disconnect, message::Message, publish};
use crate::mqtt::{
    MqttProtocolVersion, QoS, code::ReturnCode, command::ClientCommand, error::MqttProtocolError,
    helper::BrokerHelper, utils,
};

use super::store::Store;

struct ClientStream<S: AsyncRead + AsyncWrite + Unpin> {
    framed: Framed<S, MessageCodec>,
}

pub async fn process_client<S>(
    client_stream: S,
    addr: SocketAddr,
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut span = tracing::info_span!("client", %addr);
    let mut async_client = ClientStream {
        framed: tokio_util::codec::Framed::new(client_stream, MessageCodec::default()),
    };

    let mut resend_tk = time::interval(time::Duration::from_secs(
        CONFIG.get().unwrap().mqtt.settings.resend_interval,
    ));
    resend_tk.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    let mut version = MqttProtocolVersion::V3_1_1;
    let mut keep_alive = CONFIG.get().unwrap().mqtt.settings.keep_alive;
    let mut client_id = String::new();
    let mut client_rx = None;
    let mut inflight_maximum = 128u16;
    let mut pre_store = None;

    let mut client_topic_alias: HashMap<u16, String> = HashMap::new();
    let mut client_topic_alias_maximum: u16 = 0;

    let result = time::timeout(time::Duration::from_secs(3), async {
        let msg = async_client.framed.next().await;
        if msg.is_none() || msg.as_ref().unwrap().is_err() {
            debug!(parent: &span, "disconnected before CONNECT" );
            async_client.framed.close().await.ok();
            return Err(());
        }
        let msg = msg.unwrap().unwrap();
        if let Message::Connect(conn) = msg {
            span = tracing::info_span!("client", %addr, client = %g_utils::TruncateDisplay::new(&conn.client_id, 24));
            let (client_tx, c_rx) = mpsc::channel::<ClientCommand>(128);
            client_rx = Some(c_rx);

            if let Ok((ack, old_store)) = broker_helper.connect(conn.clone(), client_tx).await {
                if ack.return_code != ReturnCode::Success {
                    debug!(parent: &span, "connection rejected: {}", ack.return_code);
                    async_client
                        .framed
                        .send(Message::ConnAck(ack))
                        .await
                        .ok();
                    async_client.framed.close().await.ok();
                    return Err(());
                }
                version = conn.version;
                if conn.keep_alive > keep_alive {
                    keep_alive = conn.keep_alive;
                }
                client_id = conn.client_id.clone();
                pre_store = old_store;

                if conn.version == MqttProtocolVersion::V5 {
                    async_client.framed.codec_mut().with_v5();
                    client_topic_alias_maximum = ack.topic_alias_maximum;
                }
                inflight_maximum = conn.inflight_maximum;
                async_client.framed.codec_mut().with_packet_size(conn.packet_maximum);

                async_client
                    .framed
                    .send(Message::ConnAck(ack))
                    .await
                    .ok();
                info!(parent: &span, "connected, version: {}, keep alive: {}, new start: {}, session expiry interval: {}", conn.version, keep_alive, conn.clean_start, conn.session_expiry_interval);
            }
            return Ok(());
        } else {
            async_client.framed.close().await.ok();
            return Err(());
        }
    })
    .await;

    if result.is_err() || result.unwrap().is_err() {
        async_client.framed.close().await.ok();
        return;
    }

    let mut message_store = Store::new(
        inflight_maximum as usize,
        CONFIG.get().unwrap().mqtt.settings.max_receive_queue as usize,
    );
    if let Some(pre_store) = pre_store {
        message_store.extend(pre_store);
    }

    let mut packet_id = 1;
    let mut client_rx = client_rx.unwrap();
    let resend_time = CONFIG.get().unwrap().mqtt.settings.resend_interval;
    let mut client_msg_tm = coarsetime::Clock::now_since_epoch().as_secs();
    let mut keepalive_tk = time::interval(time::Duration::from_secs(keep_alive as u64));
    keepalive_tk.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = keepalive_tk.tick(), if coarsetime::Clock::now_since_epoch().as_secs() - client_msg_tm > (keep_alive * 3 / 2) as u64 => {
                warn!(parent: &span, "keep alive timeout, disconnecting");
                broker_helper.disconnected(client_id.as_str(), ReturnCode::KeepAliveTimeout, message_store).await.ok();
                async_client.framed.close().await.ok();
                break;
            }
            _ = resend_tk.tick(), if message_store.inflight_size() > 0 => {
                let now = coarsetime::Clock::now_since_epoch().as_secs();
                for (pkid, msg) in message_store.get_inflight_messages(now, resend_time).into_iter() {
                    if let Some(msg) = msg {
                        let msg = Message::Publish(msg);
                        let _ = async_client.framed.send(msg).await;
                    } else {
                        let _ = async_client.framed.send(Message::PubRel(publish::PubRel::new(pkid, ReturnCode::Success))).await;
                    }
                }
            }
            Some(command) = client_rx.recv() => {
                match command {
                    ClientCommand::Disconnect(code) => {
                        async_client.framed.send(Message::Disconnect(Disconnect::new(code))).await.ok();
                        async_client.framed.close().await.ok();
                        break;
                    }
                    ClientCommand::Publish{qos, retain, topic, payload, user_properties, expiry_at, subscription_identifier}=> {
                        if let Some(expiry_at) = expiry_at {
                            if expiry_at != 0 && expiry_at <= coarsetime::Clock::now_since_epoch().as_secs() {
                                continue;
                            }
                        }
                        let pid = if qos != QoS::AtMostOnce {
                            packet_id = get_packet_id(packet_id);
                            Some(packet_id)
                        } else {
                            None
                        };
                        let publish = publish::Publish::new(
                            false,
                            qos,
                            retain,
                            topic.clone(),
                            pid,
                            payload.clone(),
                            user_properties.clone(),
                        ).with_subscription_identifier(subscription_identifier);
                        if qos != QoS::AtMostOnce {
                            if message_store.inflight_insert(publish.clone()) {
                                let msg = Message::Publish(publish);
                                let _ = async_client.framed.send(msg).await;
                            }
                        } else {
                            let msg = Message::Publish(publish);
                            let _ = async_client.framed.send(msg).await;
                        }
                    }
                }
            }
            msg = async_client.framed.next() => {
                if msg.is_none() || msg.as_ref().unwrap().is_err() {
                    if msg.is_some() && msg.as_ref().unwrap().is_err() {
                        warn!(parent: &span, "error reading message: {:?}", msg.unwrap().err());
                    } else {
                        info!(parent: &span, "disconnected");
                    }
                    broker_helper.disconnected(client_id.as_str(), ReturnCode::UnspecifiedError, message_store).await.ok();
                    async_client.framed.close().await.ok();
                    break;
                }
                let msg = msg.unwrap().unwrap();
                if let Message::PacketTooLarge = msg {
                    warn!(parent: &span, "packet too large, disconnecting");
                    async_client.framed.send(Message::Disconnect(Disconnect::new(ReturnCode::PacketTooLarge))).await.ok();
                    broker_helper.disconnected(client_id.as_str(), ReturnCode::PacketTooLarge, message_store).await.ok();
                    async_client.framed.close().await.ok();
                    break;
                }

                client_msg_tm = coarsetime::Clock::now_since_epoch().as_secs();

                let result = handle_message(broker_helper.clone(), operator_helper.clone(), &mut message_store, &mut client_topic_alias, client_topic_alias_maximum, client_id.as_str(), msg).instrument(span.clone()).await;
                match result {
                    Ok(Some(resp)) => {
                        let _ = async_client.framed.send(resp).await;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!(parent: &span, "connection error : {}", e);
                        if let MqttProtocolError::Disconnected(code) = e {
                            broker_helper.disconnected(client_id.as_str(), code, message_store).await.ok();
                        } else {
                            broker_helper.disconnected(client_id.as_str(), ReturnCode::UnspecifiedError, message_store).await.ok();
                        }
                        async_client.framed.close().await.ok();
                        break;
                    }
                }
            }
        }
    }
}

pub async fn handle_message(
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
    message_store: &mut Store,
    client_topic_alias: &mut HashMap<u16, String>,
    client_topic_alias_maximum: u16,
    client_id: &str,
    msg: Message,
) -> Result<Option<Message>, MqttProtocolError> {
    match msg {
        Message::Connect(_) => {
            debug!("sent CONNECT after initial CONNECT");
            Err(MqttProtocolError::InvalidMessageType)
        }
        Message::PingReq => Ok(Some(Message::PingResp)),
        Message::Disconnect(dis) => Err(MqttProtocolError::Disconnected(dis.reason)),
        Message::Subscribe(sub) => {
            for (topic, options) in sub.topics.iter() {
                debug!(
                    "subscribe topic: {}, {}",
                    g_utils::TruncateDisplay::new(topic, 128),
                    options
                );
            }
            let ack = broker_helper.subscribe(client_id, sub).await?;
            Ok(Some(Message::SubAck(ack)))
        }
        Message::Unsubscribe(unsub) => {
            for topic in unsub.topics.iter() {
                debug!(
                    "unsubscribe topic: {}",
                    g_utils::TruncateDisplay::new(topic, 128)
                );
            }
            let ack = broker_helper.unsubscribe(client_id, unsub).await?;
            Ok(Some(Message::UnsubAck(ack)))
        }
        Message::Publish(mut publish) => {
            if let Some(topic_alias) = publish.topic_alias {
                if topic_alias == 0 {
                    return Err(MqttProtocolError::Disconnected(
                        ReturnCode::TopicAliasInvalid,
                    ));
                }

                if publish.topic.is_empty() {
                    if client_topic_alias.contains_key(&topic_alias) {
                        publish.topic = client_topic_alias.get(&topic_alias).unwrap().clone();
                    }
                } else {
                    if utils::pub_topic_valid(&publish.topic) {
                        if client_topic_alias.contains_key(&topic_alias) {
                            client_topic_alias.insert(topic_alias, publish.topic.clone());
                        } else {
                            if client_topic_alias.len() < client_topic_alias_maximum as usize {
                                client_topic_alias.insert(topic_alias, publish.topic.clone());
                            } else {
                                return Err(MqttProtocolError::Disconnected(
                                    ReturnCode::TopicAliasInvalid,
                                ));
                            }
                        }
                    } else {
                        publish.topic = String::new();
                    }
                }
            }

            if publish.topic_alias.is_some() && publish.topic.is_empty()
                || !utils::pub_topic_valid(&publish.topic)
            {
                if publish.qos == QoS::AtLeastOnce {
                    let pub_ack = publish::PubAck::new(
                        publish.packet_id.unwrap_or(0),
                        ReturnCode::TopicNameInvalid,
                    );
                    return Ok(Some(Message::PubAck(pub_ack)));
                } else if publish.qos == QoS::ExactlyOnce {
                    let pub_rec = publish::PubRec::new(
                        publish.packet_id.unwrap_or(0),
                        ReturnCode::TopicNameInvalid,
                    );
                    return Ok(Some(Message::PubRec(pub_rec)));
                } else {
                    return Ok(None);
                }
            }

            if publish.qos == QoS::AtLeastOnce {
                let pub_ack =
                    publish::PubAck::new(publish.packet_id.unwrap_or(0), ReturnCode::Success);
                if publish.retain {
                    broker_helper
                        .retain_message(
                            publish.topic.clone(),
                            publish.qos,
                            publish.payload.clone(),
                            publish.user_properties.clone(),
                            publish.expiry_at,
                        )
                        .await
                        .ok();
                }

                operator_helper
                    .publish(
                        client_id.to_string(),
                        publish.retain,
                        publish.qos,
                        publish.topic.clone(),
                        publish.payload,
                        publish.user_properties,
                        publish.expiry_at,
                    )
                    .await
                    .ok();
                Ok(Some(Message::PubAck(pub_ack)))
            } else if publish.qos == QoS::ExactlyOnce {
                if message_store.qos2_contains(publish.packet_id.unwrap()) {
                    let pub_rec =
                        publish::PubRec::new(publish.packet_id.unwrap_or(0), ReturnCode::Success);
                    return Ok(Some(Message::PubRec(pub_rec)));
                }

                let pub_rec =
                    publish::PubRec::new(publish.packet_id.unwrap_or(0), ReturnCode::Success);

                if !message_store.qos2_insert(publish) {
                    Err(MqttProtocolError::Disconnected(
                        ReturnCode::ReceiveMaximumExceeded,
                    ))
                } else {
                    Ok(Some(Message::PubRec(pub_rec)))
                }
            } else {
                if publish.retain {
                    broker_helper
                        .retain_message(
                            publish.topic.clone(),
                            publish.qos,
                            publish.payload.clone(),
                            publish.user_properties.clone(),
                            publish.expiry_at,
                        )
                        .await
                        .ok();
                }
                operator_helper
                    .publish(
                        client_id.to_string(),
                        publish.retain,
                        publish.qos,
                        publish.topic.clone(),
                        publish.payload,
                        publish.user_properties,
                        publish.expiry_at,
                    )
                    .await
                    .ok();
                Ok(None)
            }
        }
        Message::PubAck(puback) => {
            message_store.inflight_ack(puback.packet_id);
            Ok(None)
        }
        Message::PubComp(pubcomp) => {
            message_store.inflight_cmp(pubcomp.packet_id);
            Ok(None)
        }
        Message::PubRec(pubrec) => {
            message_store.inflight_rec(pubrec.packet_id);
            let pub_rel = publish::PubRel::new(pubrec.packet_id, ReturnCode::Success);
            Ok(Some(Message::PubRel(pub_rel)))
        }
        Message::PubRel(pubrel) => {
            let publish_msg = message_store.qos2_rel(pubrel.packet_id);
            let pub_comp = publish::PubComp::new(
                pubrel.packet_id,
                if publish_msg.is_none() {
                    ReturnCode::PacketIdentifierNotFound
                } else {
                    ReturnCode::Success
                },
            );
            if let Some(publish) = publish_msg {
                operator_helper
                    .publish(
                        client_id.to_string(),
                        publish.retain,
                        publish.qos,
                        publish.topic,
                        publish.payload,
                        publish.user_properties,
                        publish.expiry_at,
                    )
                    .await
                    .ok();
            }

            Ok(Some(Message::PubComp(pub_comp)))
        }
        _ => Err(MqttProtocolError::InvalidMessageType),
    }
}

pub fn get_packet_id(packet_id: u16) -> u16 {
    if packet_id == u16::MAX {
        1
    } else {
        packet_id + 1
    }
}
