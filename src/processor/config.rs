use std::collections::HashMap;
use std::sync::Arc;

use minijinja::Environment;
use serde::Deserialize;
use uuid::Uuid;
use wasmtime::Engine;

use super::{
    Processor,
    processors::{filter, json_transform, logger, republish, webhook},
    wasm::WasmProcessor,
};

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
    #[serde(rename = "webhook")]
    WebHook {
        url: String,
        method: Option<String>,
        headers: HashMap<String, String>,
        body_template: Option<String>,
        timeout_ms: Option<u64>,
        max_concurrency: Option<usize>,
    },
    #[serde(rename = "json_transform")]
    JsonTransform { template: String },
    #[serde(rename = "filter")]
    Filter {
        condition: String,
        on_error_pass: Option<bool>,
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
        env: Arc<Environment<'static>>,
    ) -> Result<Box<dyn Processor>, String> {
        match self {
            ProcessorConfig::Logger { .. } => {
                logger::LoggerProcessor::new_with_id(id, self.clone()).map_err(|e| e.to_string())
            }
            ProcessorConfig::Republish { .. } => {
                republish::RepublishProcessor::new_with_id(id, self.clone(), env)
                    .map_err(|e| e.to_string())
            }
            ProcessorConfig::WebHook { .. } => {
                webhook::WebhookProcessor::new_with_id(id, self.clone(), env)
                    .map_err(|e| e.to_string())
            }
            ProcessorConfig::JsonTransform { .. } => {
                json_transform::JsonTransformProcessor::new_with_id(id, self.clone(), env)
                    .map_err(|e| e.to_string())
            }
            ProcessorConfig::Filter { .. } => {
                filter::FilterProcessor::new_with_id(id, self.clone(), env).map_err(|e| e.to_string())
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