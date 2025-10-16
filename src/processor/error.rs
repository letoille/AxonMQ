use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Internal error")]
    InternalError,
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("WASM error: {0}")]
    WasmError(#[from] anyhow::Error),
    #[error("Processor error: {0}")]
    ProcessorError(String),
    #[error("Template error: {0}")]
    TemplateError(String),
}
