use std::any::Any;

use async_trait::async_trait;
use bytes::Bytes;
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
}

impl RepublishProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
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
                topic,
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

    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        let mut new_message = message;

        new_message.topic = self.topic.clone();

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
