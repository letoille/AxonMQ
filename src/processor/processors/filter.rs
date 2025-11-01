use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use minijinja::{Environment, context};
use serde_json::Value as JsonValue;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::processor::message::{MetadataKey, MetadataValue};

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

#[derive(Clone)]
pub struct FilterProcessor {
    id: Uuid,
    env: Arc<Environment<'static>>,
    condition: String,
    on_error_pass: bool,
}

impl FilterProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
        env: Arc<Environment<'static>>,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::Filter {
            condition,
            on_error_pass,
        } = config
        {
            let on_error_pass = on_error_pass.unwrap_or(true);
            Ok(Box::new(FilterProcessor {
                id,
                env,
                condition,
                on_error_pass,
            }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "Invalid configuration for FilterProcessor".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Processor for FilterProcessor {
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
                    warn!(error = %e, "Failed to parse payload as JSON, cannot evaluate condition.");
                    return if self.on_error_pass {
                        Ok(Some(message))
                    } else {
                        Ok(None)
                    };
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

        let render_result = self.env.render_str(&self.condition, ctx);

        match render_result {
            Ok(rendered_string) => {
                let is_true = !rendered_string.trim().is_empty()
                    && !rendered_string.trim().eq_ignore_ascii_case("false")
                    && rendered_string.trim() != "0";

                if is_true { Ok(Some(message)) } else { Ok(None) }
            }
            Err(e) => {
                warn!("Failed to render filter condition: {}", e);
                if self.on_error_pass {
                    Ok(Some(message))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
