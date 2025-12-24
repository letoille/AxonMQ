use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

use super::protocol::{
    conn::{ConnAck, Connect},
    property::PropertyUser,
    publish::PublishOptions,
    subscribe::{SubAck, Subscribe, UnsubAck, Unsubscribe},
};
use super::{QoS, code::ReturnCode};

use super::listener::store::Store;

pub(crate) enum BrokerAck {
    ConnAck(ConnAck, Option<Store>),
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
    Disconnected(String, ReturnCode, Option<u32>, Store),
    WillPublish {
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        user_properties: Vec<PropertyUser>,
        options: PublishOptions,
    },
    RetainMessage {
        topic: String,
        qos: QoS,
        payload: Bytes,
        user_properties: Vec<PropertyUser>,
        options: PublishOptions,
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
        user_properties: Vec<PropertyUser>,
        options: PublishOptions,
    },
}
