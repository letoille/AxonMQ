use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use super::code::ReturnCode;
use super::command::{BrokerCommand, ClientCommand};

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum MqttProtocolError {
    #[error("Channel send error: {0}")]
    BrokerChannelSendError(#[from] mpsc::error::SendError<BrokerCommand>),
    #[error("Channel send error: {0}")]
    StackChannelSendError(#[from] mpsc::error::SendError<ClientCommand>),
    #[error("Oneshot receive error: {0}")]
    OneshotReceiveError(#[from] oneshot::error::RecvError),
    #[error("Internal error")]
    InternalError,
    #[error("error: {0}")]
    Disconnect(#[from] std::io::Error),
    #[error("Disconnected: {0}")]
    Disconnected(ReturnCode),
    #[error("Invalid fixed header")]
    InvalidFixedHeader,
    #[error("Invalid message type")]
    InvalidMessageType,
    #[error("Invalid QoS")]
    InvalidQoS,
    #[error("Invalid protocol name")]
    InvalidProtocolName,
    #[error("Invalid protocol version")]
    InvalidProtocolVersion,
    #[error("Malformed payload")]
    MalformedPayload,
    #[error("Invalid property")]
    InvalidProperty,
    #[error("Invalid topic filter")]
    InvalidTopicFilter,
    #[error("Invalid return code: {0}")]
    InvalidReturnCode(u8),
}
