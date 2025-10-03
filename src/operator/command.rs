use bytes::Bytes;

use crate::mqtt::{QoS, protocol::property::Property};

use super::sink::Sink;

#[allow(dead_code)]
pub(crate) enum OperatorAck {}

pub(crate) enum OperatorCommand {
    Subscribe {
        client_id: String,
        share_group: Option<String>,
        topic: String,
        qos: QoS,
        no_local: bool,
        persist: bool,
        sink: Box<dyn Sink>,
    },
    Unsubscribe {
        client_id: String,
        share_group: Option<String>,
        topic: String,
    },
    RemoveClient {
        client_id: String,
    },
    Publish {
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    },
}
