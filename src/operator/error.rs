use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use super::command::OperatorCommand;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub(crate) enum OperatorError {
    #[error("Internal error")]
    InternalError,
    #[error("Channel send error: {0}")]
    ChannelSendError(#[from] mpsc::error::SendError<OperatorCommand>),
    #[error("Oneshot receive error: {0}")]
    OneshotReceiveError(#[from] oneshot::error::RecvError),
}
