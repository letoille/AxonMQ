use std::sync::Arc;

use serde::Deserialize;
use uuid::Uuid;
use wasmtime::Engine;

use super::{Processor, processors::{logger, republish}, wasm::WasmProcessor};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ProcessorConfig {
    #[serde(rename = "logger")]
    Logger { level: String },
    #[serde(rename = "republish")]
    Republish {
        topic: String,
        qos: Option<u8>,
        retain: Option<bool>,
        payload: Option<String>,
    },
    #[serde(rename = "wasm")]
    Wasm { path: String, cfg: String },
    #[serde(other)]
    Other,
}

impl ProcessorConfig {
    pub async fn new_processor(
        &self,
        id: Uuid,
        engine: Arc<Engine>,
    ) -> Result<Box<dyn Processor>, String> {
        match self {
            ProcessorConfig::Logger { .. } => {
                logger::LoggerProcessor::new_with_id(id, self.clone()).map_err(|e| e.to_string())
            }
            ProcessorConfig::Republish { .. } => {
                republish::RepublishProcessor::new_with_id(id, self.clone()).map_err(|e| e.to_string())
            }
            ProcessorConfig::Wasm { path, cfg } => {
                WasmProcessor::new(engine, path.clone(), id, cfg.to_string())
                    .await
                    .map_err(|e| e.to_string())
            }
            ProcessorConfig::Other => Err("Unsupported processor type".to_string()),
        }
    }
}
