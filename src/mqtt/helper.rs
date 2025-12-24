use tokio::sync::{mpsc, oneshot};

use super::QoS;
use super::code::ReturnCode;
use super::command::{BrokerAck, BrokerCommand, ClientCommand};
use super::error::MqttProtocolError;
use super::listener::store::Store;
use super::protocol::{
    conn::{ConnAck, Connect},
    property::PropertyUser,
    publish::PublishOptions,
    subscribe::{SubAck, Subscribe, UnsubAck, Unsubscribe},
};

#[derive(Clone)]
pub struct BrokerHelper {
    pub broker_tx: mpsc::Sender<BrokerCommand>,
}

#[derive(Clone)]
pub struct ClientHelper {
    pub client_tx: mpsc::Sender<ClientCommand>,
}

impl BrokerHelper {
    pub async fn connect(
        &self,
        connect: Connect,
        client_tx: mpsc::Sender<ClientCommand>,
    ) -> Result<(ConnAck, Option<Store>), MqttProtocolError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.broker_tx
            .send(BrokerCommand::Connect {
                connect,
                resp: resp_tx,
                client_tx,
            })
            .await
            .map_err(|_| MqttProtocolError::BrokerChannelSendError)?;

        let result = resp_rx.await?;
        if let BrokerAck::ConnAck(ack, store) = result {
            Ok((ack, store))
        } else {
            Err(MqttProtocolError::InternalError)
        }
    }

    pub async fn subscribe(
        &self,
        client_id: &str,
        subscribe: Subscribe,
    ) -> Result<SubAck, MqttProtocolError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.broker_tx
            .send(BrokerCommand::Subscribe {
                client_id: client_id.to_string(),
                subscribe,
                resp: resp_tx,
            })
            .await
            .map_err(|_| MqttProtocolError::BrokerChannelSendError)?;

        let result = resp_rx.await?;
        if let BrokerAck::SubAck(ack) = result {
            Ok(ack)
        } else {
            Err(MqttProtocolError::InternalError)
        }
    }

    pub async fn unsubscribe(
        &self,
        client_id: &str,
        unsubscribe: Unsubscribe,
    ) -> Result<UnsubAck, MqttProtocolError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.broker_tx
            .send(BrokerCommand::Unsubscribe {
                client_id: client_id.to_string(),
                unsubscribe,
                resp: resp_tx,
            })
            .await
            .map_err(|_| MqttProtocolError::BrokerChannelSendError)?;

        let result = resp_rx.await?;
        if let BrokerAck::UnsubAck(ack) = result {
            Ok(ack)
        } else {
            Err(MqttProtocolError::InternalError)
        }
    }

    pub async fn disconnected(
        &self,
        client_id: &str,
        code: ReturnCode,
        session_expiry_interval: Option<u32>,
        store: Store,
    ) -> Result<(), MqttProtocolError> {
        self.broker_tx
            .send(BrokerCommand::Disconnected(
                client_id.to_string(),
                code,
                session_expiry_interval,
                store,
            ))
            .await
            .map_err(|_| MqttProtocolError::BrokerChannelSendError)?;
        Ok(())
    }

    pub async fn will_publish(
        &self,
        client_id: &str,
        topic: String,
        payload: bytes::Bytes,
        qos: QoS,
        retain: bool,
        user_properties: Vec<PropertyUser>,
        options: PublishOptions,
    ) -> Result<(), MqttProtocolError> {
        self.broker_tx
            .send(BrokerCommand::WillPublish {
                client_id: client_id.to_string(),
                topic,
                payload,
                qos,
                retain,
                user_properties,
                options,
            })
            .await
            .map_err(|_| MqttProtocolError::BrokerChannelSendError)?;
        Ok(())
    }

    pub async fn retain_message(
        &self,
        topic: String,
        qos: QoS,
        payload: bytes::Bytes,
        user_properties: Vec<PropertyUser>,
        options: PublishOptions,
    ) -> Result<(), MqttProtocolError> {
        self.broker_tx
            .send(BrokerCommand::RetainMessage {
                topic,
                qos,
                payload,
                user_properties,
                options,
            })
            .await
            .map_err(|_| MqttProtocolError::BrokerChannelSendError)?;
        Ok(())
    }

    pub fn store_msg(&self, client_id: &str, msg: ClientCommand) -> Result<(), MqttProtocolError> {
        let _ = self.broker_tx.try_send(BrokerCommand::StoreMsg {
            client_id: client_id.to_string(),
            msg,
        });
        Ok(())
    }
}

impl ClientHelper {
    pub async fn disconnect(&self, reason: ReturnCode) -> Result<(), MqttProtocolError> {
        self.client_tx
            .send(ClientCommand::Disconnect(reason))
            .await?;
        Ok(())
    }

    pub fn new(stack_tx: mpsc::Sender<ClientCommand>) -> Self {
        ClientHelper {
            client_tx: stack_tx,
        }
    }

    pub fn get(&self) -> mpsc::Sender<ClientCommand> {
        self.client_tx.clone()
    }

    pub fn send(&self, cmd: ClientCommand) -> Result<(), MqttProtocolError> {
        self.client_tx
            .try_send(cmd)
            .map_err(|_| MqttProtocolError::InternalError)
    }
}
