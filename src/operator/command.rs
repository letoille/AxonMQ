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

impl std::fmt::Display for OperatorCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorCommand::Subscribe {
                client_id, topic, ..
            } => {
                write!(f, "Subscribe: client_id={}, topic={}", client_id, topic)
            }
            OperatorCommand::Unsubscribe {
                client_id, topic, ..
            } => {
                write!(f, "Unsubscribe: client_id={}, topic={}", client_id, topic)
            }
            OperatorCommand::RemoveClient { client_id } => {
                write!(f, "RemoveClient: client_id={}", client_id)
            }
            OperatorCommand::Publish {
                client_id, topic, ..
            } => {
                write!(f, "Publish: client_id={}, topic={}", client_id, topic)
            }
        }
    }
}
