use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use minijinja::{Environment, context};
use serde_json::Value as JsonValue;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::mqtt::QoS;
use crate::processor::message::{MetadataKey, MetadataValue};

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

#[derive(Clone)]
pub struct RepublishProcessor {
    id: Uuid,
    env: Arc<Environment<'static>>,
    topic_template: String,
    qos: Option<QoS>,
    retain: Option<bool>,
    payload: Option<Bytes>,
}

impl RepublishProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
        env: Arc<Environment<'static>>,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::Republish {
            topic,
            qos,
            retain,
            payload,
        } = config
        {
            let qos = qos.map(|q| QoS::try_from(q).unwrap_or(QoS::AtMostOnce));
            let payload = payload.map(|p| Bytes::from(p));

            Ok(Box::new(RepublishProcessor {
                id,
                env,
                topic_template: topic,
                qos,
                retain,
                payload,
            }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "Invalid configuration for RepublishProcessor".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Processor for RepublishProcessor {
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
                Err(_) => JsonValue::Null,
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

        let final_topic = self
            .env
            .render_str(&self.topic_template, ctx)
            .map_err(|e| {
                warn!("Failed to render republish topic template: {}", e);
                ProcessorError::TemplateError(e.to_string())
            })?;

        message.topic = final_topic;

        if let Some(qos) = self.qos {
            message.qos = qos;
        }

        if let Some(retain) = self.retain {
            message.retain = retain;
        }

        if let Some(payload) = &self.payload {
            message.payload = payload.clone();
        }

        Ok(Some(message))
    }
}
