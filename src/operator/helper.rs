use bytes::Bytes;
use tokio::sync::mpsc;

use crate::mqtt::QoS;
use crate::mqtt::protocol::property::Property;

use super::command::{OperatorCommand, OutSender};
use super::error::OperatorError;
use super::out::mqtt::MqttOutSender;

#[derive(Clone)]
pub struct Helper<T: OutSender> {
    pub operator_tx: mpsc::Sender<OperatorCommand<T>>,
}

impl Helper<MqttOutSender> {
    pub async fn subscribe(
        &self,
        client_id: String,
        topic: String,
        qos: QoS,
        no_local: bool,
        store: bool,
        sender: MqttOutSender,
    ) -> Result<(), OperatorError<MqttOutSender>> {
        self.operator_tx
            .send(OperatorCommand::MqttSubscribe {
                client_id,
                topic,
                qos,
                no_local,
                store,
                sender,
            })
            .await
            .map_err(Into::into)
    }

    pub async fn unsubscribe(
        &self,
        client_id: String,
        topic: String,
    ) -> Result<(), OperatorError<MqttOutSender>> {
        self.operator_tx
            .send(OperatorCommand::MqttUnsubscribe { client_id, topic })
            .await
            .map_err(Into::into)
    }

    pub async fn remove_client(
        &self,
        client_id: String,
    ) -> Result<(), OperatorError<MqttOutSender>> {
        self.operator_tx
            .send(OperatorCommand::MqttRemoveClient { client_id })
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
    ) -> Result<(), OperatorError<MqttOutSender>> {
        self.operator_tx
            .send(OperatorCommand::MqttPublish {
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

    pub async fn purge_expiry(&self) -> Result<(), OperatorError<MqttOutSender>> {
        self.operator_tx
            .send(OperatorCommand::MqttPurgeExpiry)
            .await
            .map_err(Into::into)
    }
}
