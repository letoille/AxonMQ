use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Internal error")]
    InternalError,
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}
