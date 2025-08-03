use tokio::sync::mpsc;

use super::super::command::OutSender;
use crate::mqtt::command::ClientCommand;

#[derive(Clone)]
pub(crate) struct MqttOutSender {
    pub(crate) tx: mpsc::Sender<ClientCommand>,
}

impl MqttOutSender {
    pub fn new(tx: mpsc::Sender<ClientCommand>) -> Self {
        MqttOutSender { tx }
    }
}

impl std::default::Default for MqttOutSender {
    fn default() -> Self {
        let (tx, _rx) = mpsc::channel(1);
        MqttOutSender { tx }
    }
}

impl OutSender for MqttOutSender {
    type Message = ClientCommand;

    fn send(
        &self,
        msg: Self::Message,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<Self::Message>> {
        self.tx.try_send(msg)
    }
}
