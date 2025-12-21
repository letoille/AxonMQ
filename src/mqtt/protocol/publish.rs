use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt as _};
use bytes::{BufMut, Bytes, BytesMut};

use super::super::{MqttProtocolVersion, QoS, code::ReturnCode, error::MqttProtocolError};
use super::{
    message::Message,
    property::{Property, PropertyUser},
};

#[derive(Clone)]
pub struct Publish {
    pub(crate) dup: bool,
    pub(crate) qos: QoS,
    pub(crate) retain: bool,

    pub(crate) topic: String,
    pub(crate) payload: Bytes,

    pub(crate) topic_alias: Option<u16>,
    pub(crate) packet_id: Option<u16>,
    pub(crate) expiry_at: Option<u64>,
    pub(crate) subscription_identifier: Option<u32>,

    pub(crate) user_properties: Vec<PropertyUser>,
}

#[derive(Clone)]
pub struct PubAck {
    pub(crate) packet_id: u16,
    pub(crate) reason_code: ReturnCode,
}

pub type PubRec = PubAck;
pub type PubRel = PubAck;
pub type PubComp = PubAck;

impl Publish {
    pub fn new(
        dup: bool,
        qos: QoS,
        retain: bool,
        topic: String,
        packet_id: Option<u16>,
        payload: Bytes,
        user_properties: Vec<PropertyUser>,
    ) -> Self {
        Publish {
            dup,
            qos,
            retain,
            topic,
            packet_id,
            payload,
            expiry_at: None,
            topic_alias: None,
            subscription_identifier: None,
            user_properties,
        }
    }

    pub fn with_subscription_identifier(mut self, subscription_identifier: Option<u32>) -> Self {
        self.subscription_identifier = subscription_identifier;
        self
    }

    pub fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(2 + self.topic.len() + self.payload.len());

        buf.put_u16(self.topic.len() as u16);
        buf.put(self.topic.as_bytes());

        if self.qos != QoS::AtMostOnce {
            if let Some(packet_id) = self.packet_id {
                buf.put_u16(packet_id);
            } else {
                buf.put_u16(0);
            }
        }

        if version == MqttProtocolVersion::V5 {
            let mut prop_bytes = BytesMut::new();
            for prop in self.user_properties {
                let prop = Property::UserProperty(prop);
                prop_bytes.put(prop.into_bytes());
            }

            let prop_len = prop_bytes.len();
            if prop_len < 128 {
                buf.put_u8(prop_len as u8);
            } else if prop_len < 16384 {
                buf.put_u8(((prop_len % 128) as u8) | 0x80);
                buf.put_u8((prop_len / 128) as u8);
            } else if prop_len < 2097152 {
                buf.put_u8(((prop_len % 128) as u8) | 0x80);
                buf.put_u8((((prop_len / 128) % 128) as u8) | 0x80);
                buf.put_u8((prop_len / 16384) as u8);
            } else {
                buf.put_u8(((prop_len % 128) as u8) | 0x80);
                buf.put_u8((((prop_len / 128) % 128) as u8) | 0x80);
                buf.put_u8((((prop_len / 16384) % 128) as u8) | 0x80);
                buf.put_u8((prop_len / 2097152) as u8);
            }
            buf.put(prop_bytes);
        }

        buf.put(self.payload);

        buf.freeze()
    }

    pub fn publish_try_from(
        rdr: &mut Cursor<Bytes>,
        version: MqttProtocolVersion,
        qos: QoS,
        dup: bool,
        retain: bool,
    ) -> Result<Message, MqttProtocolError> {
        let topic_len = rdr.read_u16::<BigEndian>()?;
        let mut topic = vec![0u8; topic_len as usize];

        let topic = if topic_len > 0 {
            rdr.read_exact(&mut topic)?;
            String::from_utf8(topic).map_err(|_| MqttProtocolError::InvalidTopicFilter)?
        } else {
            String::new()
        };

        let packet_id = if qos != QoS::AtMostOnce {
            let pid = rdr.read_u16::<BigEndian>()?;
            Some(pid)
        } else {
            None
        };

        let mut expiry_at = None;
        let mut topic_alias: Option<u16> = None;
        let mut subscription_identifier = None;
        let mut user_properties = Vec::new();

        if version == MqttProtocolVersion::V5 {
            let properties = Property::try_from_properties(rdr)?;
            for prop in properties.into_iter() {
                match prop {
                    Property::MessageExpiryInterval(v) => {
                        expiry_at = Some(coarsetime::Clock::now_since_epoch().as_secs() + v as u64);
                    }
                    Property::TopicAlias(v) => {
                        topic_alias = Some(v);
                    }
                    Property::SubscriptionIdentifier(v) => {
                        subscription_identifier = Some(v);
                    }
                    Property::UserProperty(v) => {
                        user_properties.push(v);
                    }
                    _ => {}
                }
            }
        };

        let end_offset = rdr.position() as usize;
        let payload = rdr.get_ref().slice(end_offset..);

        let publish = Publish {
            dup,
            qos,
            retain,
            topic,
            packet_id,
            payload,
            expiry_at,
            topic_alias,
            subscription_identifier,
            user_properties,
        };

        Ok(Message::Publish(publish))
    }
}

impl PubAck {
    pub fn new(packet_id: u16, code: ReturnCode) -> Self {
        PubAck {
            packet_id,
            reason_code: code,
        }
    }

    pub fn puback_try_from(
        rdr: &mut std::io::Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        let packet_id = rdr.read_u16::<BigEndian>()?;
        if version == MqttProtocolVersion::V3_1_1 || version == MqttProtocolVersion::V3 {
            return Ok(Message::PubAck(PubAck {
                packet_id,
                reason_code: ReturnCode::Success,
            }));
        }

        let reason_code = rdr.read_u8()?;
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut _properties = vec![];
        let properties_len = rdr.read_u8().unwrap_or(0);
        if properties_len > 0 {
            _properties = Property::try_from_properties(rdr)?;
        }

        Ok(Message::PubAck(PubAck {
            packet_id,
            reason_code,
        }))
    }

    pub fn pubrec_try_from(
        rdr: &mut std::io::Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        let packet_id = rdr.read_u16::<BigEndian>()?;
        if version == MqttProtocolVersion::V3_1_1 || version == MqttProtocolVersion::V3 {
            return Ok(Message::PubRec(PubAck {
                packet_id,
                reason_code: ReturnCode::Success,
            }));
        }

        let reason_code = rdr.read_u8().unwrap_or(ReturnCode::Success as u8);
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut _properties = vec![];
        if reason_code != ReturnCode::Success {
            let properties_len = rdr.read_u8().unwrap_or(0);
            if properties_len > 0 {
                _properties = Property::try_from_properties(rdr)?;
            }
        }

        Ok(Message::PubRec(PubAck {
            packet_id,
            reason_code,
        }))
    }

    pub fn pubcomp_try_from(
        rdr: &mut std::io::Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        let packet_id = rdr.read_u16::<BigEndian>()?;
        if version == MqttProtocolVersion::V3_1_1 || version == MqttProtocolVersion::V3 {
            return Ok(Message::PubComp(PubAck {
                packet_id,
                reason_code: ReturnCode::Success,
            }));
        }

        let reason_code = rdr.read_u8().unwrap_or(ReturnCode::Success as u8);
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut _properties = vec![];
        if reason_code != ReturnCode::Success {
            let properties_len = rdr.read_u8().unwrap_or(0);
            if properties_len > 0 {
                _properties = Property::try_from_properties(rdr)?;
            }
        }

        Ok(Message::PubComp(PubAck {
            packet_id,
            reason_code,
        }))
    }

    pub fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(4);

        buf.put_u16(self.packet_id);

        if version == MqttProtocolVersion::V5 {
            if self.reason_code != ReturnCode::Success {
                buf.put_u8(self.reason_code as u8);
                buf.put_u8(0);
                //buf.put_u8(self.properties.len() as u8);
                //for prop in self.properties {
                //buf.put(prop.into_bytes());
                //}
            }
        }

        buf.freeze()
    }

    pub fn pubrel_try_from(
        rdr: &mut std::io::Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        let packet_id = rdr.read_u16::<BigEndian>()?;
        if version == MqttProtocolVersion::V3_1_1 || version == MqttProtocolVersion::V3 {
            return Ok(Message::PubRel(PubRel {
                packet_id,
                reason_code: ReturnCode::Success,
            }));
        }

        let reason_code = rdr.read_u8().unwrap_or(ReturnCode::Success as u8);
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut _properties = vec![];
        if reason_code != ReturnCode::Success {
            let properties_len = rdr.read_u8().unwrap_or(0);
            if properties_len > 0 {
                _properties = Property::try_from_properties(rdr)?;
            }
        }

        Ok(Message::PubRel(PubRel {
            packet_id,
            reason_code,
        }))
    }
}
