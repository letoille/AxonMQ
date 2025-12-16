use tokio::sync::mpsc::{Sender, error::TrySendError};
use tracing::{debug, trace, warn};

use crate::mqtt::{QoS, command::ClientCommand, helper::BrokerHelper};
use crate::processor::message::Message;

use super::Sink;

#[derive(Clone)]
pub struct LocalClientSink {
    sender: Sender<ClientCommand>,
    broker_helper: BrokerHelper,
}

impl LocalClientSink {
    pub fn new(sender: Sender<ClientCommand>, broker_helper: BrokerHelper) -> Box<Self> {
        Box::new(LocalClientSink {
            sender,
            broker_helper,
        })
    }
}

impl Sink for LocalClientSink {
    fn deliver(&self, message: Message, persist: bool) {
        let msg = ClientCommand::Publish {
            topic: message.topic,
            qos: message.qos,
            retain: message.retain,
            payload: message.payload,
            properties: message.properties,
            expiry_at: message.expiry_at,
        };

        if persist && message.qos != QoS::AtMostOnce {
            if self.sender.try_send(msg.clone()).is_err() {
                if let Err(e) = self.broker_helper.store_msg(&message.client_id, msg) {
                    warn!(
                        "failed to store message for client {}: {}",
                        message.client_id, e
                    );
                }
            }
        } else {
            if let Err(e) = self.sender.try_send(msg) {
                match e {
                    TrySendError::Full(_) => {
                        debug!(
                            "message queue full for client {}, dropping message",
                            message.client_id
                        );
                    }
                    TrySendError::Closed(_) => {
                        trace!(
                            "client {} disconnected, dropping message",
                            message.client_id
                        );
                    }
                }
            }
        }
    }
}
