use bytes::Bytes;
use tokio::sync::mpsc;

use crate::mqtt::QoS;
use crate::mqtt::protocol::property::Property;

use super::command::OperatorCommand;
use super::error::OperatorError;
use super::sink::Sink;

#[derive(Clone)]
pub struct Helper {
    operator_tx: mpsc::Sender<OperatorCommand>,
}

impl Helper {
    pub fn new(operator_tx: mpsc::Sender<OperatorCommand>) -> Self {
        Helper { operator_tx }
    }

    pub async fn subscribe(
        &self,
        client_id: String,
        share_group: Option<String>,
        topic: String,
        qos: QoS,
        no_local: bool,
        persist: bool,
        sink: Box<dyn Sink>,
    ) -> Result<(), OperatorError> {
        self.operator_tx
            .send(OperatorCommand::Subscribe {
                client_id,
                share_group,
                topic,
                qos,
                no_local,
                persist,
                sink,
            })
            .await
            .map_err(Into::into)
    }

    pub async fn unsubscribe(
        &self,
        client_id: String,
        share_group: Option<String>,
        topic: String,
    ) -> Result<(), OperatorError> {
        self.operator_tx
            .send(OperatorCommand::Unsubscribe {
                client_id,
                share_group,
                topic,
            })
            .await
            .map_err(Into::into)
    }

    pub async fn remove_client(&self, client_id: String) -> Result<(), OperatorError> {
        self.operator_tx
            .send(OperatorCommand::RemoveClient { client_id })
            .await
            .map_err(Into::into)
    }

    pub async fn publish(
        &self,
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
        properties: Vec<Property>,
        expiry_at: Option<u64>,
    ) -> Result<(), OperatorError> {
        self.operator_tx
            .send(OperatorCommand::Publish {
                client_id,
                retain,
                qos,
                topic,
                payload,
                properties,
                expiry_at,
            })
            .await
            .map_err(Into::into)
    }
}
