use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use minijinja::{Environment, Value, context};
use tracing::{info, instrument, warn};
use uuid::Uuid;

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
            // Default to true (pass on error) if not specified
            let on_error_pass = on_error_pass.unwrap_or(true);
            Ok(Box::new(FilterProcessor {
                id,
                env,
                condition,
                on_error_pass,
            }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "invalid configuration for FilterProcessor".to_string(),
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
    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        let payload_json: Value = match serde_json::from_slice(&message.payload) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    client_id = %message.client_id,
                    topic = %message.topic,
                    "failed to parse payload as JSON, cannot evaluate condition. Error: {}",
                    e
                );
                return if self.on_error_pass {
                    Ok(Some(message))
                } else {
                    Ok(None)
                };
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
        };

        let render_result = self.env.render_str(&self.condition, ctx);

        match render_result {
            Ok(rendered_string) => {
                // We consider a string that is not case-insensitively equal to "false" or "0" as true.
                // An empty string from the template is considered false.
                info!("filter condition evaluated {}", rendered_string);
                let is_true = !rendered_string.trim().is_empty()
                    && !rendered_string.trim().eq_ignore_ascii_case("false")
                    && rendered_string.trim() != "0";

                if is_true {
                    // Condition is true, PASS the message through
                    Ok(Some(message))
                } else {
                    // Condition is false, DROP the message
                    Ok(None)
                }
            }
            Err(e) => {
                warn!("failed to render filter condition: {}", e);
                if self.on_error_pass {
                    Ok(Some(message))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
