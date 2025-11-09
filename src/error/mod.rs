use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::service::sparkplug_b::in_helper::InMessage;

#[derive(Debug, Error)]
pub enum AxonError {
    #[error("SparkPlug B Error: {0}")]
    SparkPlugBError(#[from] crate::service::sparkplug_b::error::SpbError),

    #[error("Timeout Error: {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("Channel Send SparkPlug B Error: {0}")]
    ChannelSendError(#[from] mpsc::error::SendError<InMessage>),

    #[error("Oneshot Receive Error: {0}")]
    OneshotRecvError(#[from] oneshot::error::RecvError),
}
