use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use minijinja::{Environment, context};
use serde_json::Value as JsonValue;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::processor::message::{MetadataKey, MetadataPayloadFormat, MetadataValue};

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
        let payload_json = if let Some(MetadataValue::Json(val)) = message
            .metadata
            .get(MetadataKey::ParsedPayloadJson.as_str())
        {
            val.clone()
        } else {
            let parsed_val: JsonValue = match serde_json::from_slice(&message.payload) {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = %e, "Failed to parse payload as JSON, passing through without transformation.");
                    return Ok(Some(message));
                }
            };
            message.metadata.insert(
                MetadataKey::ParsedPayloadJson.as_str().to_string(),
                MetadataValue::Json(parsed_val.clone()),
            );
            parsed_val
        };

        let ctx = context! {
            topic => message.topic.clone(),
            client_id => message.client_id.clone(),
            qos => message.qos as u8,
            retain => message.retain,
            payload => payload_json,
            metadata => message.metadata.clone(),
        };

        let new_payload_str = self.env.render_str(&self.template, ctx).map_err(|e| {
            warn!("Failed to render template: {}", e);
            ProcessorError::TemplateError(e.to_string())
        })?;

        message.payload = Bytes::from(new_payload_str.clone());

        match serde_json::from_str::<JsonValue>(&new_payload_str) {
            Ok(new_json) => {
                message.metadata.insert(
                    MetadataKey::ParsedPayloadJson.as_str().to_string(),
                    MetadataValue::Json(new_json),
                );
                message.metadata.insert(
                    MetadataKey::PayloadFormat.as_str().to_string(),
                    MetadataValue::String(MetadataPayloadFormat::Json.as_str().to_string()),
                );
            }
            Err(_) => {
                message
                    .metadata
                    .remove(MetadataKey::ParsedPayloadJson.as_str());
                message.metadata.insert(
                    MetadataKey::PayloadFormat.as_str().to_string(),
                    MetadataValue::String(MetadataPayloadFormat::String.as_str().to_string()),
                );
            }
        }

        Ok(Some(message))
    }
}
