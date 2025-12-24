use bytes::Bytes;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::CONFIG;
use crate::mqtt::QoS;
use crate::mqtt::protocol::{property::PropertyUser, publish::PublishOptions};
use crate::utils::time::now_milliseconds;

use super::command::OperatorCommand;
use super::error::OperatorError;
use super::sink::Sink;

#[derive(Clone)]
pub struct Helper {
    matcher_tx: mpsc::Sender<OperatorCommand>,
    router_tx: mpsc::Sender<OperatorCommand>,
}

#[derive(Serialize)]
struct StateMessage {
    pub online: bool,
    pub timestamp: u64,
}

impl Helper {
    pub fn new(
        matcher_tx: mpsc::Sender<OperatorCommand>,
        router_tx: mpsc::Sender<OperatorCommand>,
    ) -> Self {
        Helper {
            matcher_tx,
            router_tx,
        }
    }

    pub async fn subscribe(
        &self,
        client_id: String,
        share_group: Option<String>,
        topic: String,
        qos: QoS,
        no_local: bool,
        subscription_id: Option<u32>,
        persist: bool,
        sink: Box<dyn Sink>,
    ) -> Result<(), OperatorError> {
        self.matcher_tx
            .send(OperatorCommand::Subscribe {
                client_id,
                share_group,
                topic,
                qos,
                no_local,
                subscription_id,
                persist,
                sink,
            })
            .await
            .map_err(|e| OperatorError::ChannelSendError(e.to_string()))
    }

    pub async fn unsubscribe(
        &self,
        client_id: String,
        share_group: Option<String>,
        topic: String,
    ) -> Result<(), OperatorError> {
        self.matcher_tx
            .send(OperatorCommand::Unsubscribe {
                client_id,
                share_group,
                topic,
            })
            .await
            .map_err(|e| OperatorError::ChannelSendError(e.to_string()))
    }

    pub async fn remove_client(&self, client_id: String) -> Result<(), OperatorError> {
        self.matcher_tx
            .send(OperatorCommand::RemoveClient { client_id })
            .await
            .map_err(|e| OperatorError::ChannelSendError(e.to_string()))
    }

    pub async fn publish(
        &self,
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        user_properties: Vec<PropertyUser>,
        options: PublishOptions,
    ) -> Result<(), OperatorError> {
        self.router_tx
            .send(OperatorCommand::Publish {
                client_id,
                retain,
                qos,
                topic,
                payload,
                user_properties,
                options,
            })
            .await
            .map_err(|e| OperatorError::ChannelSendError(e.to_string()))
    }

    pub async fn sparkplug_b_publish(
        &self,
        topic: String,
        payload: Bytes,
    ) -> Result<(), OperatorError> {
        self.router_tx
            .send(OperatorCommand::SparkPlugBPublish {
                client_id: "sparkplug_b_application".to_string(),
                topic,
                payload,
                retain: false,
                qos: QoS::AtMostOnce,
            })
            .await
            .map_err(|e| OperatorError::ChannelSendError(e.to_string()))
    }

    pub async fn sparkplug_b_state_online(&self) -> Result<(), OperatorError> {
        let topic = format!(
            "spBv1.0/STATE/{}",
            CONFIG.get().unwrap().service.sparkplug_b.application_id
        );
        let payload = StateMessage {
            online: true,
            timestamp: now_milliseconds(),
        };

        self.router_tx
            .send(OperatorCommand::SparkPlugBPublish {
                client_id: "sparkplug_b_application".to_string(),
                topic,
                payload: Bytes::from(serde_json::to_vec(&payload).unwrap()),
                retain: true,
                qos: QoS::AtLeastOnce,
            })
            .await
            .map_err(|e| OperatorError::ChannelSendError(e.to_string()))
    }
}
