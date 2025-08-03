use bytes::Bytes;

use crate::mqtt::{QoS, protocol::property::Property};

pub(crate) trait OutSender {
    type Message;

    fn send(
        &self,
        msg: Self::Message,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<Self::Message>>;
}

#[allow(dead_code)]
pub(crate) enum OperatorAck {}

pub(crate) enum OperatorCommand<T: OutSender> {
    MqttSubscribe {
        client_id: String,
        topic: String,
        qos: QoS,
        no_local: bool,
        store: bool,
        sender: T,
    },
    MqttUnsubscribe {
        client_id: String,
        topic: String,
    },
    MqttRemoveClient {
        client_id: String,
    },
    MqttPurgeExpiry,
    MqttPublish {
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    },
}
