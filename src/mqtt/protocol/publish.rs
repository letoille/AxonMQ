use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt as _};
use bytes::{BufMut, Bytes, BytesMut};

use super::super::{code::ReturnCode, error::MqttProtocolError, MqttProtocolVersion, QoS};
use super::{message::Message, property::Property};

#[derive(Clone)]
pub struct Publish {
    pub(crate) dup: bool,
    pub(crate) qos: QoS,
    pub(crate) retain: bool,

    pub(crate) topic: String,
    pub(crate) packet_id: Option<u16>,
    pub(crate) payload: Bytes,

    pub(crate) properties: Vec<Property>,
    pub(crate) expiry_at: Option<u64>,
}

#[derive(Clone)]
pub struct PubAck {
    pub(crate) packet_id: u16,
    pub(crate) reason_code: ReturnCode,
    pub(crate) properties: Vec<Property>,
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
        properties: Vec<Property>,
    ) -> Self {
        Publish {
            dup,
            qos,
            retain,
            topic,
            packet_id,
            payload,
            properties,
            expiry_at: None,
        }
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
            for prop in self.properties {
                prop_bytes.put(prop.into_bytes());
            }

            let prop_len = prop_bytes.len();
            if prop_len < 128 {
                buf.put_u8(prop_len as u8);
            } else if prop_len < 32768 {
                buf.put_u8(((prop_len % 128) as u8) | 0x80);
                buf.put_u8((prop_len / 128) as u8);
            } else if prop_len < 8388608 {
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
        rdr.read_exact(&mut topic)?;

        let topic = String::from_utf8(topic).map_err(|_| MqttProtocolError::InvalidTopicName)?;

        let packet_id = if qos != QoS::AtMostOnce {
            let pid = rdr.read_u16::<BigEndian>()?;
            Some(pid)
        } else {
            None
        };

        let mut properties = Vec::new();
        let mut expiry_at = None;
        if version == MqttProtocolVersion::V5 {
            properties = Property::try_from_properties(rdr)?;
            for prop in properties.iter() {
                if let Property::MessageExpiryInterval(v) = prop {
                    expiry_at = Some(coarsetime::Instant::now().elapsed().as_secs() + *v as u64);
                }
            }
            properties.retain(|p| !matches!(p, Property::MessageExpiryInterval(_)));
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
            properties,
            expiry_at,
        };

        Ok(Message::Publish(publish))
    }
}

impl PubAck {
    pub fn new(packet_id: u16, code: ReturnCode, properties: Vec<Property>) -> Self {
        PubAck {
            packet_id,
            reason_code: code,
            properties,
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
                properties: vec![],
            }));
        }

        let reason_code = rdr.read_u8()?;
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut properties = vec![];
        let properties_len = rdr.read_u8()?;
        if properties_len > 0 {
            properties = Property::try_from_properties(rdr)?;
        }

        Ok(Message::PubAck(PubAck {
            packet_id,
            reason_code,
            properties,
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
                properties: vec![],
            }));
        }

        let reason_code = rdr.read_u8().unwrap_or(ReturnCode::Success as u8);
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut properties = vec![];
        if reason_code != ReturnCode::Success {
            let properties_len = rdr.read_u8()?;
            if properties_len > 0 {
                properties = Property::try_from_properties(rdr)?;
            }
        }

        Ok(Message::PubRec(PubAck {
            packet_id,
            reason_code,
            properties,
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
                properties: vec![],
            }));
        }

        let reason_code = rdr.read_u8().unwrap_or(ReturnCode::Success as u8);
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut properties = vec![];
        if reason_code != ReturnCode::Success {
            let properties_len = rdr.read_u8()?;
            if properties_len > 0 {
                properties = Property::try_from_properties(rdr)?;
            }
        }

        Ok(Message::PubComp(PubAck {
            packet_id,
            reason_code,
            properties,
        }))
    }

    pub fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(4 + self.properties.len());

        buf.put_u16(self.packet_id);

        if version == MqttProtocolVersion::V5 {
            if self.reason_code != ReturnCode::Success || !self.properties.is_empty() {
                buf.put_u8(self.reason_code as u8);
                buf.put_u8(self.properties.len() as u8);
                for prop in self.properties {
                    buf.put(prop.into_bytes());
                }
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
                properties: vec![],
            }));
        }

        let reason_code = rdr.read_u8().unwrap_or(ReturnCode::Success as u8);
        let reason_code = ReturnCode::try_from(reason_code)?;

        let mut properties = vec![];
        if reason_code != ReturnCode::Success {
            let properties_len = rdr.read_u8()?;
            if properties_len > 0 {
                properties = Property::try_from_properties(rdr)?;
            }
        }

        Ok(Message::PubRel(PubRel {
            packet_id,
            reason_code,
            properties,
        }))
    }
}
