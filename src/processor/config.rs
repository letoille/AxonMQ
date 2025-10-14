use serde::Deserialize;
use uuid::Uuid;

use super::{Processor, processors::logger};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ProcessorConfig {
    #[serde(rename = "logger")]
    Logger { level: String },
    #[serde(other)]
    Other,
}

impl ProcessorConfig {
    pub fn new_processor(&self, id: Uuid) -> Result<Box<dyn Processor>, String> {
        match self {
            ProcessorConfig::Logger { .. } => {
                logger::LoggerProcessor::new_with_id(id, self.clone()).map_err(|e| e.to_string())
            }
            ProcessorConfig::Other => Err("Unsupported processor type".to_string()),
        }
    }
}
