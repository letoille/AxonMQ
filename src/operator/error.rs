use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use super::command::{OperatorCommand, OutSender};

#[allow(dead_code)]
#[derive(Error)]
pub(crate) enum OperatorError<T: OutSender> {
    #[error("Internal error")]
    InternalError,
    #[error("Channel send error: {0}")]
    ChannelSendError(#[from] mpsc::error::SendError<OperatorCommand<T>>),
    #[error("Oneshot receive error: {0}")]
    OneshotReceiveError(#[from] oneshot::error::RecvError),
}
