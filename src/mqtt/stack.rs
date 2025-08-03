use coarsetime;
use futures_util::{SinkExt, stream::StreamExt as _};
use tokio::{net::TcpListener, sync::mpsc, time};
use tokio_util::codec::Framed;
use tracing::{Instrument, debug, info};

use crate::CONFIG;
use crate::operator::{helper::Helper, out::mqtt::MqttOutSender};
use crate::utils as g_utils;

use super::protocol::{codec::MessageCodec, conn::Disconnect, message::Message, publish};
use super::{
    MqttProtocolVersion, QoS, code::ReturnCode, command::ClientCommand, error::MqttProtocolError,
    helper::BrokerHelper, utils,
};

struct AsyncServerSteam {
    framed: Framed<tokio::net::TcpStream, MessageCodec>,
}

pub struct Stack {
    listener: TcpListener,
    broker_helper: BrokerHelper,
    operator_helper: Helper<MqttOutSender>,
}

impl Stack {
    pub async fn new(
        host: &str,
        port: u16,
        broker_helper: BrokerHelper,
        operator_helper: Helper<MqttOutSender>,
    ) -> Self {
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(addr).await.unwrap();

        Stack {
            listener,
            broker_helper,
            operator_helper,
        }
    }

    pub async fn start(&mut self) {
        loop {
            let (new_client, client_addr) = self.listener.accept().await.unwrap();
            self.process_client(
                new_client,
                client_addr,
                self.broker_helper.clone(),
                self.operator_helper.clone(),
            );
        }
    }

    async fn handle_message(
        broker_helper: BrokerHelper,
        operator_helper: Helper<MqttOutSender>,
        inflight_store: &mut Vec<(u64, Message)>,
        qos2_msg_store: &mut Vec<publish::Publish>,
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
            Message::Publish(publish) => {
                if !utils::pub_topic_valid(&publish.topic) {
                    if publish.qos == QoS::AtLeastOnce {
                        let pub_ack = publish::PubAck::new(
                            publish.packet_id.unwrap_or(0),
                            ReturnCode::TopicFilterInvalid,
                            vec![],
                        );
                        return Ok(Some(Message::PubAck(pub_ack)));
                    } else if publish.qos == QoS::ExactlyOnce {
                        let pub_rec = publish::PubRec::new(
                            publish.packet_id.unwrap_or(0),
                            ReturnCode::TopicFilterInvalid,
                            vec![],
                        );
                        return Ok(Some(Message::PubRec(pub_rec)));
                    } else {
                        return Ok(None);
                    }
                }
                if publish.qos == QoS::AtLeastOnce {
                    let pub_ack = publish::PubAck::new(
                        publish.packet_id.unwrap_or(0),
                        ReturnCode::Success,
                        vec![],
                    );
                    operator_helper
                        .publish(
                            client_id.to_string(),
                            publish.retain,
                            publish.qos,
                            publish.topic.clone(),
                            publish.payload,
                            publish.properties,
                            publish.expiry_at,
                        )
                        .await
                        .ok();
                    Ok(Some(Message::PubAck(pub_ack)))
                } else if publish.qos == QoS::ExactlyOnce {
                    let pub_rec = publish::PubRec::new(
                        publish.packet_id.unwrap_or(0),
                        ReturnCode::Success,
                        vec![],
                    );
                    if qos2_msg_store.len()
                        >= CONFIG.get().unwrap().mqtt.settings.max_receive_queue as usize
                    {
                        qos2_msg_store.remove(0);
                    }
                    qos2_msg_store.push(publish);
                    Ok(Some(Message::PubRec(pub_rec)))
                } else {
                    operator_helper
                        .publish(
                            client_id.to_string(),
                            publish.retain,
                            publish.qos,
                            publish.topic.clone(),
                            publish.payload,
                            publish.properties,
                            publish.expiry_at,
                        )
                        .await
                        .ok();
                    Ok(None)
                }
            }
            Message::PubAck(puback) => {
                inflight_store.retain(|(_, m)| {
                    if let Message::Publish(p) = m {
                        if p.packet_id == Some(puback.packet_id) {
                            return false;
                        }
                    }
                    true
                });
                Ok(None)
            }
            Message::PubComp(pubcomp) => {
                inflight_store.retain(|(_, m)| {
                    if let Message::Publish(p) = m {
                        if p.packet_id == Some(pubcomp.packet_id) {
                            return false;
                        }
                    }
                    true
                });
                Ok(None)
            }
            Message::PubRec(pubrec) => {
                let pub_rel = publish::PubRel::new(pubrec.packet_id, ReturnCode::Success, vec![]);
                Ok(Some(Message::PubRel(pub_rel)))
            }
            Message::PubRel(pubrel) => {
                let mut pub_comp =
                    publish::PubComp::new(pubrel.packet_id, ReturnCode::Success, vec![]);
                let mut found = false;
                for (i, p) in qos2_msg_store.iter().enumerate() {
                    if p.packet_id == Some(pubrel.packet_id) {
                        operator_helper
                            .publish(
                                client_id.to_string(),
                                p.retain,
                                p.qos,
                                p.topic.clone(),
                                p.payload.clone(),
                                p.properties.clone(),
                                p.expiry_at,
                            )
                            .await
                            .ok();
                        qos2_msg_store.remove(i);
                        found = true;
                        break;
                    }
                }
                if !found {
                    pub_comp.reason_code = ReturnCode::PacketIdentifierNotFound;
                }
                Ok(Some(Message::PubComp(pub_comp)))
            }
            _ => Err(MqttProtocolError::InvalidMessageType),
        }
    }

    fn get_packet_id(packet_id: u16) -> u16 {
        if packet_id == u16::MAX {
            1
        } else {
            packet_id + 1
        }
    }

    fn process_client(
        &self,
        client: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        broker_helper: BrokerHelper,
        operator_helper: Helper<MqttOutSender>,
    ) {
        tokio::spawn(async move {
            let mut span = tracing::info_span!("client", %addr);
            let mut async_client = AsyncServerSteam {
                framed: tokio_util::codec::Framed::new(client, MessageCodec::default()),
            };
            let mut resend_tk = time::interval(time::Duration::from_secs(
                CONFIG.get().unwrap().mqtt.settings.resend_interval,
            ));
            resend_tk.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

            let mut version = MqttProtocolVersion::V3_1_1;
            let mut keep_alive = 60u16;
            let mut client_id = String::new();
            let mut client_rx = None;
            let mut inflight_maximum = 128u16;

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

                    if let Ok(ack) = broker_helper.connect(conn.clone(), client_tx).await {
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
                        keep_alive = conn.keep_alive;
                        client_id = conn.client_id.clone();

                        if conn.version == MqttProtocolVersion::V5 {
                            async_client.framed.codec_mut().with_v5();
                        }
                        inflight_maximum = conn.inflight_maximum.unwrap_or(128);
                        async_client.framed.codec_mut().with_packet_size(conn.packet_maximum.unwrap_or(2 * 1024 * 1024));

                        async_client
                            .framed
                            .send(Message::ConnAck(ack))
                            .await
                            .ok();
                        debug!(parent: &span, "connected, version: {}, keep alive: {}, new start: {}", conn.version, keep_alive, conn.clean_start);
                    }
                    return Ok(());
                } else {
                    info!(parent: &span, "first message not CONNECT, closing");
                    async_client.framed.close().await.ok();
                    return Err(());
                }
            })
            .await;

            if result.is_err() || result.unwrap().is_err() {
                info!(parent: &span, "client did not send CONNECT in time");
                async_client.framed.close().await.ok();
                return;
            }

            let mut packet_id = 1;
            let mut inflight_store: Vec<(u64, Message)> = Vec::new();
            let mut client_rx = client_rx.unwrap();
            let mut qos2_msg_store: Vec<publish::Publish> = Vec::new();

            loop {
                tokio::select! {
                    _ = resend_tk.tick(), if inflight_store.len() > 0 => {
                        let now = coarsetime::Instant::now().elapsed().as_secs();
                        for (tm, msg) in inflight_store.iter_mut() {
                            if *tm + 2 < now {
                                let _ = async_client.framed.send(msg.clone()).await;
                                *tm = now;
                            }
                        }
                    }
                    Some(command) = client_rx.recv(), if inflight_store.len() < inflight_maximum as usize => {
                        match command {
                            ClientCommand::Disconnect(code) => {
                                async_client.framed.send(Message::Disconnect(Disconnect::new(code) )).await.ok();
                                async_client.framed.close().await.ok();
                                break;
                            }
                            ClientCommand::Publish{qos, retain, topic, payload, properties, expiry_at} => {
                                if let Some(expiry_at) = expiry_at {
                                    if expiry_at <= coarsetime::Instant::now().elapsed().as_secs() {
                                        continue;
                                    }
                                }
                                let pid = if qos != QoS::AtMostOnce {
                                    packet_id = Self::get_packet_id(packet_id);
                                    Some(packet_id)
                                } else {
                                    None
                                };
                                let msg = Message::Publish(publish::Publish::new(
                                    false,
                                    qos,
                                    retain,
                                    topic.clone(),
                                    pid,
                                    payload.clone(),
                                    properties.clone(),
                                ));
                                if qos != QoS::AtMostOnce {
                                    if inflight_store.len() >= inflight_maximum as usize {
                                        let _ = inflight_store.remove(0);
                                    }
                                    let mut msg = msg.clone();
                                    msg.with_dup();
                                    inflight_store.push((coarsetime::Instant::now().elapsed().as_secs(), msg));
                                }
                                let _ = async_client.framed.send(msg).await;
                            }
                        }
                    }
                    msg = async_client.framed.next() => {
                        if msg.is_none() || msg.as_ref().unwrap().is_err() {
                            debug!(parent: &span, "disconnected");
                            broker_helper.disconnected(client_id.as_str(), ReturnCode::UnspecifiedError).await.ok();
                            async_client.framed.close().await.ok();
                            break;
                        }
                        let msg = msg.unwrap().unwrap();
                        if let Message::PacketTooLarge = msg {
                            debug!(parent: &span, "packet too large, disconnecting");
                            async_client.framed.send(Message::Disconnect(Disconnect::new(ReturnCode::PacketTooLarge))).await.ok();
                            broker_helper.disconnected(client_id.as_str(), ReturnCode::PacketTooLarge).await.ok();
                            async_client.framed.close().await.ok();
                            break;
                        }

                        let result = Self::handle_message(broker_helper.clone(), operator_helper.clone(), &mut inflight_store, &mut qos2_msg_store, client_id.as_str(), msg).instrument(span.clone()).await;
                        match result {
                            Ok(Some(resp)) => {
                                let _ = async_client.framed.send(resp).await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                debug!(parent: &span, "error handling message: {}", e);
                                if let MqttProtocolError::Disconnected(code) = e {
                                    broker_helper.disconnected(client_id.as_str(), code).await.ok();
                                } else {
                                    broker_helper.disconnected(client_id.as_str(), ReturnCode::UnspecifiedError).await.ok();
                                }
                                async_client.framed.close().await.ok();
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}
