use std::any::Any;

use async_trait::async_trait;
use tracing::{debug, error, info, instrument, trace, warn};
use uuid::Uuid;

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

#[derive(Clone)]
pub struct LoggerProcessor {
    id: Uuid,
    level: String,
}

impl LoggerProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::Logger { level } = config {
            Ok(Box::new(LoggerProcessor { id, level }))
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

    #[instrument(skip(self, message), fields(id = %self.id))]
    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        match self.level.as_str() {
            "trace" => trace!("{}", message),
            "debug" => debug!("{}", message),
            "info" => info!("{}", message),
            "warn" => warn!("{}", message),
            "error" => error!("{}", message),
            _ => unreachable!("Invalid log level"),
        };
        Ok(Some(message))
    }
}
