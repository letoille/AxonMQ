use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use minijinja::{Environment, context};
use tracing::warn;
use uuid::Uuid;

use crate::mqtt::QoS;

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

#[derive(Clone)]
pub struct RepublishProcessor {
    id: Uuid,
    topic: String,
    qos: Option<QoS>,
    retain: Option<bool>,
    payload: Option<Bytes>,

    is_topic_dynamic: bool,
    env: Arc<Environment<'static>>,
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
            let is_topic_dynamic = topic.contains("{{") || topic.contains("{%");

            Ok(Box::new(RepublishProcessor {
                id,
                topic,
                qos,
                retain,
                payload,
                env,
                is_topic_dynamic,
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

    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        let mut new_message = message;

        if self.is_topic_dynamic {
            let ctx = context! {
                topic => new_message.topic.clone(),
                client_id => new_message.client_id.clone(),
                qos => new_message.qos as u8,
                retain => new_message.retain,
                payload => String::from_utf8_lossy(&new_message.payload).to_string(),
                properties => new_message.properties.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
            };

            let topic = self.env.render_str(&self.topic, ctx).map_err(|e| {
                warn!("failed to render topic template: {}", e);
                ProcessorError::TemplateError(e.to_string())
            })?;
            new_message.topic = topic;
        } else {
            new_message.topic = self.topic.clone();
        }

        if let Some(qos) = self.qos {
            new_message.qos = qos;
        }

        if let Some(retain) = self.retain {
            new_message.retain = retain;
        }

        if let Some(payload) = &self.payload {
            new_message.payload = payload.clone();
        }

        Ok(Some(new_message))
    }
}
