use std::any::Any;

use async_trait::async_trait;
use tracing::{Span, debug, error, info, trace, warn};
use uuid::Uuid;

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

#[derive(Clone)]
pub struct LoggerProcessor {
    id: Uuid,
    level: String,
    span: Span,
}

impl LoggerProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::Logger { level } = config {
            let span = match level.as_str() {
                "trace" => tracing::trace_span!("Logger", id = %id),
                "debug" => tracing::debug_span!("Logger", id = %id),
                "info" => tracing::info_span!("Logger", id = %id),
                "warn" => tracing::warn_span!("Logger", id = %id),
                "error" => tracing::error_span!("Logger", id = %id),
                _ => {
                    return Err(ProcessorError::InvalidConfiguration(
                        "Invalid log level".to_string(),
                    ));
                }
            };
            Ok(Box::new(LoggerProcessor { id, level, span }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "Invalid configuration for LoggerProcessor".to_string(),
            ))
        }
    }

    #[allow(dead_code)]
    pub fn new(config: ProcessorConfig) -> Result<(Uuid, Box<dyn Processor>), ProcessorError> {
        let id = Uuid::new_v4();
        Ok((id, LoggerProcessor::new_with_id(id, config)?))
    }
}

#[async_trait]
impl Processor for LoggerProcessor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        match self.level.as_str() {
            "trace" => trace!(parent: &self.span, "{}", message),
            "debug" => debug!(parent: &self.span, "{}", message),
            "info" => info!(parent: &self.span, "{}", message),
            "warn" => warn!(parent: &self.span, "{}", message),
            "error" => error!(parent: &self.span, "{}", message),
            _ => unreachable!("Invalid log level"),
        };
        Ok(Some(message))
    }
}
