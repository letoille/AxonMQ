use bytes::Bytes;
use tokio::sync::mpsc::Sender;

use crate::mqtt::QoS;

use super::error::SpbError;
use super::message::Message;
use super::proto;

pub(crate) struct Publish {
    #[allow(dead_code)]
    pub(crate) client_id: String,
    pub(crate) retain: bool,
    pub(crate) qos: QoS,
    pub(crate) topic: String,
    pub(crate) payload_bytes: Option<Bytes>,

    pub(crate) payload: Option<proto::Payload>,
    pub(crate) gn: Option<(String, String)>,
}

impl Publish {
    pub fn new(client_id: String, retain: bool, qos: QoS, topic: String, payload: Bytes) -> Self {
        Publish {
            client_id,
            retain,
            qos,
            topic,
            payload_bytes: Some(payload),

            payload: None,
            gn: None,
        }
    }

    pub fn parse(&mut self) -> Result<Message, SpbError> {
        use prost::Message as _;

        let message;
        let parts: Vec<&str> = self.topic.split('/').collect();
        if parts.len() != 3 && parts.len() != 4 && parts.len() != 5 {
            return Err(SpbError::InvalidTopic);
        }

        let payload = proto::Payload::decode(self.payload_bytes.take().unwrap())
            .map_err(|_| SpbError::InvalidPayload)?;

        match parts[2] {
            "NBIRTH" => {
                if parts.len() != 4 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                message = Message::parse_nbirth(parts[1], parts[3], &payload)?;
            }
            "NDEATH" => {
                if parts.len() != 4 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtLeastOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                message = Message::parse_ndeath(parts[1], parts[3], &payload)?;
            }
            "NDATA" => {
                if parts.len() != 4 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                self.gn = Some((parts[1].to_string(), parts[3].to_string()));
                message = Message::parse_ndata(parts[1], parts[3], &payload)?;
            }
            "NCMD" => {
                if parts.len() != 4 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                message = Message::parse_ncmd(parts[1], parts[3], &payload)?;
            }
            "DBIRTH" => {
                if parts.len() != 5 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                self.gn = Some((parts[1].to_string(), parts[3].to_string()));
                message = Message::parse_dbirth(parts[1], parts[3], parts[4], &payload)?;
            }
            "DDEATH" => {
                if parts.len() != 5 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                self.gn = Some((parts[1].to_string(), parts[3].to_string()));
                message = Message::parse_ddeath(parts[1], parts[3], parts[4], &payload)?;
            }
            "DDATA" => {
                if parts.len() != 5 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                self.gn = Some((parts[1].to_string(), parts[3].to_string()));
                message = Message::parse_ddata(parts[1], parts[3], parts[4], &payload)?;
            }
            "DCMD" => {
                if parts.len() != 5 {
                    return Err(SpbError::InvalidTopic);
                }

                if self.qos != QoS::AtMostOnce {
                    return Err(SpbError::InvalidQoS);
                }

                if self.retain {
                    return Err(SpbError::InvalidRetain);
                }

                message = Message::parse_dcmd(parts[1], parts[3], parts[4], &payload)?;
            }
            _ => {
                return Err(SpbError::InvalidTopic);
            }
        }

        self.payload = Some(payload);
        Ok(message)
    }
}

#[derive(Clone)]
pub struct SparkPlugBApplicationHelper {
    tx: Sender<Publish>,
}

impl SparkPlugBApplicationHelper {
    pub fn new(tx: Sender<Publish>) -> Self {
        SparkPlugBApplicationHelper { tx }
    }

    pub fn is_sparkplug_b_topic(&self, topic: &str) -> bool {
        if topic.starts_with("spBv1.0/") {
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() != 3 && parts.len() != 4 && parts.len() != 5 {
                return false;
            }
            if parts[2] == "NBIRTH"
                || parts[2] == "NDEATH"
                || parts[2] == "NDATA"
                || parts[2] == "NCMD"
                || parts[2] == "DBIRTH"
                || parts[2] == "DDEATH"
                || parts[2] == "DDATA"
                || parts[2] == "DCMD"
                || parts[1] == "STATE"
            {
                return true;
            }
        }
        return false;
    }

    pub async fn publish(
        &self,
        client_id: String,
        retain: bool,
        qos: QoS,
        topic: String,
        payload: Bytes,
    ) {
        let publish = Publish::new(client_id, retain, qos, topic, payload);
        self.tx.send(publish).await.ok();
    }
}
