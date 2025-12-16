use std::net::SocketAddr;

use bytes::BytesMut;
use coarsetime;
use futures_util::{SinkExt, stream::StreamExt as _};
use http::StatusCode;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::tungstenite::{
    Message as WsMessage,
    handshake::server::{Request, Response},
};
use tracing::{Instrument, debug, error, info, warn};

use crate::CONFIG;
use crate::mqtt::{
    MqttProtocolVersion, QoS,
    code::ReturnCode,
    command::ClientCommand,
    error::MqttProtocolError,
    helper::BrokerHelper,
    protocol::{codec::MessageCodec, conn::Disconnect, message::Message, publish},
};
use crate::operator::helper::Helper as OperatorHelper;
use crate::utils as g_utils;

use super::shared::{get_packet_id, handle_message};
use super::store::Store;
use super::tcp::load_tls_acceptor;

use tokio_util::codec::{Decoder, Encoder};

pub fn spawn_ws_listener(
    host: String,
    port: u16,
    path: String,
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
) {
    tokio::spawn(async move {
        let addr = format!("{}:{}", host, port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("failed to bind WS listener on {}: {}", addr, e);
                return;
            }
        };
        info!("MQTT WebSocket listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            let broker_helper = broker_helper.clone();
            let operator_helper = operator_helper.clone();
            let path = path.clone();

            let callback = move |req: &Request, res: Response| {
                if req.uri().path() != path {
                    error!("WS connection rejected for path: {}", req.uri().path());
                    return Err(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(None)
                        .unwrap());
                }

                let headers = req.headers();
                let has_mqtt_subprotocol = headers
                    .get("Sec-WebSocket-Protocol")
                    .and_then(|val| val.to_str().ok())
                    .map(|protocols| protocols.split(',').any(|p| p.trim() == "mqtt"))
                    .unwrap_or(false);

                if has_mqtt_subprotocol {
                    let (mut parts, body) = res.into_parts();
                    parts
                        .headers
                        .append("Sec-WebSocket-Protocol", "mqtt".parse().unwrap());
                    Ok(Response::from_parts(parts, body))
                } else {
                    // If the client doesn't request the mqtt subprotocol, we can choose to reject or proceed.
                    // For better compatibility, we will proceed without adding the header.
                    Ok(res)
                }
            };

            tokio::spawn(async move {
                match tokio_tungstenite::accept_hdr_async(stream, callback).await {
                    Ok(ws_stream) => {
                        handle_websocket_connection(
                            ws_stream,
                            addr,
                            broker_helper,
                            operator_helper,
                        )
                        .await;
                    }
                    Err(e) => {
                        debug!("WebSocket handshake error from {}: {}", addr, e);
                    }
                }
            });
        }
    });
}

pub fn spawn_wss_listener(
    host: String,
    port: u16,
    path: String,
    cert_path: String,
    key_path: String,
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
) {
    tokio::spawn(async move {
        let addr = format!("{}:{}", host, port);
        let tls_acceptor = match load_tls_acceptor(&cert_path, &key_path) {
            Ok(acceptor) => acceptor,
            Err(e) => {
                error!("failed to load TLS config for {}: {}", addr, e);
                return;
            }
        };

        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("failed to bind WSS listener on {}: {}", addr, e);
                return;
            }
        };
        info!("MQTT Secure WebSocket listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            let acceptor = tls_acceptor.clone();
            let broker_helper = broker_helper.clone();
            let operator_helper = operator_helper.clone();
            let path = path.clone();

            let callback = move |req: &Request, res: Response| {
                if req.uri().path() != path {
                    error!("WSS connection rejected for path: {}", req.uri().path());
                    return Err(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(None)
                        .unwrap());
                }

                let headers = req.headers();
                let has_mqtt_subprotocol = headers
                    .get("Sec-WebSocket-Protocol")
                    .and_then(|val| val.to_str().ok())
                    .map(|protocols| protocols.split(',').any(|p| p.trim() == "mqtt"))
                    .unwrap_or(false);

                if has_mqtt_subprotocol {
                    let (mut parts, body) = res.into_parts();
                    parts
                        .headers
                        .append("Sec-WebSocket-Protocol", "mqtt".parse().unwrap());
                    Ok(Response::from_parts(parts, body))
                } else {
                    Ok(res)
                }
            };

            tokio::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        match tokio_tungstenite::accept_hdr_async(tls_stream, callback).await {
                            Ok(ws_stream) => {
                                handle_websocket_connection(
                                    ws_stream,
                                    addr,
                                    broker_helper,
                                    operator_helper,
                                )
                                .await;
                            }
                            Err(e) => {
                                debug!("WebSocket handshake error over TLS from {}: {}", addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("TLS handshake error from {}: {}", addr, e);
                    }
                }
            });
        }
    });
}

async fn handle_websocket_connection<S>(
    mut ws_stream: tokio_tungstenite::WebSocketStream<S>,
    addr: SocketAddr,
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut span = tracing::info_span!("client", %addr, protocol = "ws");
    let mut codec = MessageCodec::default();
    let mut read_buf = BytesMut::new();

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

    let result = time::timeout(time::Duration::from_secs(3), async {
        loop {
            if let Some(Ok(ws_msg)) = ws_stream.next().await {
                if let WsMessage::Binary(data) = ws_msg {
                    read_buf.extend_from_slice(&data);
                    loop {
                        match codec.decode(&mut read_buf) {
                            Ok(Some(Message::Connect(conn))) => {
                                span = tracing::info_span!("client", %addr, client = %g_utils::TruncateDisplay::new(&conn.client_id, 24));
                                let (client_tx, c_rx) = mpsc::channel::<ClientCommand>(128);
                                client_rx = Some(c_rx);

                                if let Ok((ack, old_store)) = broker_helper.connect(conn.clone(), client_tx).await {
                                    if ack.return_code != ReturnCode::Success {
                                        debug!(parent: &span, "connection rejected: {}", ack.return_code);
                                        let mut write_buf = BytesMut::new();
                                        codec.encode(Message::ConnAck(ack), &mut write_buf).unwrap();
                                        ws_stream.send(WsMessage::Binary(write_buf.freeze())).await.ok();
                                        ws_stream.close(None).await.ok();
                                        return Err(());
                                    }
                                    version = conn.version;
                                    if conn.keep_alive > keep_alive {
                                        keep_alive = conn.keep_alive;
                                    }
                                    client_id = conn.client_id.clone();
                                    pre_store = old_store;

                                    if conn.version == MqttProtocolVersion::V5 {
                                        codec.with_v5();
                                    }
                                    inflight_maximum = conn.inflight_maximum;
                                    codec.with_packet_size(conn.packet_maximum);

                                    let mut write_buf = BytesMut::new();
                                    codec.encode(Message::ConnAck(ack), &mut write_buf).unwrap();
                                    ws_stream.send(WsMessage::Binary(write_buf.freeze())).await.ok();
                                    debug!(parent: &span, "connected, version: {}, keep alive: {}, new start: {}, session expiry interval: {}", conn.version, keep_alive, conn.clean_start, conn.session_expiry_interval);
                                    return Ok(()); // Handshake successful
                                }
                            }
                            Ok(Some(_)) => {
                                // Received a non-CONNECT message during handshake
                                return Err(());
                            }
                            Ok(None) => {
                                // Need more data, break inner loop and wait for next ws_msg
                                break;
                            }
                            Err(_) => {
                                debug!(parent: &span, "failed to decode CONNECT message during handshake");
                                return Err(());
                            }
                        }
                    }
                } else if let WsMessage::Close(_) = ws_msg {
                    debug!(parent: &span, "client closed connection during handshake");
                    return Err(()); // Client closed connection
                }
            } else {
                debug!(parent: &span, "client disconnected during handshake");
                return Err(());
            }
        }
    })
    .await;

    if result.is_err() || result.unwrap().is_err() {
        ws_stream.close(None).await.ok();
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
                ws_stream.close(None).await.ok();
                break;
            }
            _ = resend_tk.tick(), if message_store.inflight_size() > 0 => {
                let now = coarsetime::Clock::now_since_epoch().as_secs();
                for (pkid, msg) in message_store.get_inflight_messages(now, resend_time).into_iter() {
                    if let Some(msg) = msg {
                        let mut msg = Message::Publish(msg);
                        msg.with_dup();
                        let mut write_buf = BytesMut::new();
                        codec.encode(msg.clone(), &mut write_buf).unwrap();
                        let _ = ws_stream.send(WsMessage::Binary(write_buf.freeze())).await;
                    } else {
                        let mut write_buf = BytesMut::new();
                        codec.encode(Message::PubRel(publish::PubRel::new(pkid, ReturnCode::Success, vec![])), &mut write_buf).unwrap();
                        let _ = ws_stream.send(WsMessage::Binary(write_buf.freeze())).await;
                    }
                }
            }
            Some(command) = client_rx.recv() => {
                match command {
                    ClientCommand::Disconnect(code) => {
                        let mut write_buf = BytesMut::new();
                        codec.encode(Message::Disconnect(Disconnect::new(code)), &mut write_buf).unwrap();
                        ws_stream.send(WsMessage::Binary(write_buf.freeze())).await.ok();
                        ws_stream.close(None).await.ok();
                        break;
                    }
                    ClientCommand::Publish{qos, retain, topic, payload, properties, expiry_at} => {
                        if let Some(expiry_at) = expiry_at {
                            if expiry_at <= coarsetime::Clock::now_since_epoch().as_secs() {
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
                            properties.clone(),
                        );
                        if qos != QoS::AtMostOnce {
                            message_store.inflight_insert(publish.clone());
                        }
                        let msg = Message::Publish(publish);
                        let mut write_buf = BytesMut::new();
                        codec.encode(msg, &mut write_buf).unwrap();
                        let _ = ws_stream.send(WsMessage::Binary(write_buf.freeze())).await;
                    }
                }
            }
            Some(Ok(ws_msg)) = ws_stream.next() => {
                match ws_msg {
                    WsMessage::Binary(data) => {
                        read_buf.extend_from_slice(&data);
                        loop {
                            match codec.decode(&mut read_buf) {
                                Ok(Some(msg)) => {
                                    if let Message::PacketTooLarge = msg {
                                        debug!(parent: &span, "packet too large, disconnecting");
                                        let mut write_buf = BytesMut::new();
                                        codec.encode(Message::Disconnect(Disconnect::new(ReturnCode::PacketTooLarge)), &mut write_buf).unwrap();
                                        ws_stream.send(WsMessage::Binary(write_buf.freeze())).await.ok();
                                        broker_helper.disconnected(client_id.as_str(), ReturnCode::PacketTooLarge, message_store.clone()).await.ok();
                                        ws_stream.close(None).await.ok();
                                        break;
                                    }

                                    client_msg_tm = coarsetime::Clock::now_since_epoch().as_secs();

                                    let result = handle_message(broker_helper.clone(), operator_helper.clone(), &mut message_store, client_id.as_str(), msg).instrument(span.clone()).await;
                                    match result {
                                        Ok(Some(resp)) => {
                                            let mut write_buf = BytesMut::new();
                                            codec.encode(resp, &mut write_buf).unwrap();
                                            let _ = ws_stream.send(WsMessage::Binary(write_buf.freeze())).await;
                                        }
                                        Ok(None) => {}
                                        Err(e) => {
                                            debug!(parent: &span, "error handling message: {}", e);
                                            if let MqttProtocolError::Disconnected(code) = e {
                                                broker_helper.disconnected(client_id.as_str(), code, message_store.clone()).await.ok();
                                            } else {
                                                broker_helper.disconnected(client_id.as_str(), ReturnCode::UnspecifiedError, message_store.clone()).await.ok();
                                            }
                                            ws_stream.close(None).await.ok();
                                            break;
                                        }
                                    }
                                }
                                Ok(None) => break,
                                Err(_) => {
                                    // Decode error
                                    ws_stream.close(None).await.ok();
                                    break;
                                }
                            }
                        }
                    }
                    WsMessage::Close(_) => {
                        debug!(parent: &span, "client closed connection");
                        broker_helper.disconnected(client_id.as_str(), ReturnCode::Success, message_store).await.ok();
                        break;
                    }
                    WsMessage::Ping(data) => {
                        ws_stream.send(WsMessage::Pong(data)).await.ok();
                    }
                    _ => {}
                }
            }
            else => {
                broker_helper.disconnected(client_id.as_str(), ReturnCode::UnspecifiedError, message_store).await.ok();
                break;
            }
        }
    }
}
