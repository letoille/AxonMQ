use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use minijinja::{Environment, context};
use serde_json::Value;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::utils;

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

#[derive(Clone)]
pub struct JsonTransformProcessor {
    id: Uuid,
    env: Arc<Environment<'static>>,
    template: String,
}

impl JsonTransformProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
        env: Arc<Environment<'static>>,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::JsonTransform { template } = config {
            Ok(Box::new(JsonTransformProcessor { id, env, template }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "Invalid configuration for JsonTransformProcessor".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Processor for JsonTransformProcessor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(skip(self, message), fields(id = %self.id))]
    async fn on_message(&self, mut message: Message) -> Result<Option<Message>, ProcessorError> {
        let payload_json: Value = match serde_json::from_slice(&message.payload) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "failed to parse payload as JSON, topic: {}, Error: {}",
                    utils::TruncateDisplay::new(&message.topic, 128),
                    e
                );
                // If payload is not valid JSON, pass it through without transformation.
                return Ok(Some(message));
            }
        };

        let raw_payload = String::from_utf8_lossy(&message.payload).to_string();

        let ctx = context! {
            topic => message.topic.clone(),
            client_id => message.client_id.clone(),
            qos => message.qos as u8,
            retain => message.retain,
            payload => payload_json,
            raw_payload => raw_payload,
            properties => message.properties.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
        };

        let new_payload_str = self.env.render_str(&self.template, ctx).map_err(|e| {
            warn!("failed to render template: {}", e);
            ProcessorError::TemplateError(e.to_string())
        })?;

        // Update the message with the new payload and metadata
        message.payload = Bytes::from(new_payload_str);

        Ok(Some(message))
    }
}
