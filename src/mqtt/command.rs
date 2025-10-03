use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

use super::protocol::{
    conn::{ConnAck, Connect},
    property::Property,
    subscribe::{SubAck, Subscribe, UnsubAck, Unsubscribe},
};
use super::{QoS, code::ReturnCode};

pub(crate) enum BrokerAck {
    ConnAck(ConnAck),
    SubAck(SubAck),
    UnsubAck(UnsubAck),
}

pub(crate) enum BrokerCommand {
    Connect {
        connect: Connect,
        resp: oneshot::Sender<BrokerAck>,
        client_tx: mpsc::Sender<ClientCommand>,
    },
    Subscribe {
        client_id: String,
        subscribe: Subscribe,
        resp: oneshot::Sender<BrokerAck>,
    },
    Unsubscribe {
        client_id: String,
        unsubscribe: Unsubscribe,
        resp: oneshot::Sender<BrokerAck>,
    },
    Disconnected(String, ReturnCode),
    WillPublish {
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    },
    RetainMessage {
        topic: String,
        qos: QoS,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    },
    StoreMsg {
        client_id: String,
        msg: ClientCommand,
    },
}

#[derive(Clone)]
pub enum ClientCommand {
    Disconnect(ReturnCode),
    Publish {
        topic: String,
        qos: QoS,
        retain: bool,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    },
}
