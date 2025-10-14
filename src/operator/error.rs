use thiserror::Error;
use tokio::sync::oneshot;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum OperatorError {
    #[error("Internal error")]
    InternalError,
    #[error("Channel send error: {0}")]
    ChannelSendError(String),
    #[error("Oneshot receive error: {0}")]
    OneshotReceiveError(#[from] oneshot::error::RecvError),
}
