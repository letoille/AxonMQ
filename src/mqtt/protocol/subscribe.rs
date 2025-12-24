use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt as _};
use bytes::{BufMut, Bytes, BytesMut};

use super::super::{MqttProtocolVersion, QoS, code::ReturnCode, error::MqttProtocolError};
use super::{message::Message, property::Property};

#[derive(Clone)]
pub struct SubscribeOption {
    pub(crate) qos: QoS,
    pub(crate) no_local: bool,
    pub(crate) retain_as_published: bool,
    pub(crate) retain_handling: u8,

    pub(crate) subscription_identifier: Option<u32>,
}

#[derive(Clone)]
pub struct Subscribe {
    pub(crate) packet_id: u16,
    pub(crate) topics: Vec<(String, SubscribeOption)>,
}

#[derive(Clone)]
pub struct SubAck {
    pub(crate) packet_id: u16,
    pub(crate) return_codes: Vec<ReturnCode>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Unsubscribe {
    pub(crate) packet_id: u16,
    pub(crate) topics: Vec<String>,
}

#[derive(Clone)]
pub struct UnsubAck {
    pub(crate) packet_id: u16,
    pub(crate) return_codes: Vec<ReturnCode>,
}

impl SubAck {
    pub fn new(packet_id: u16, return_codes: Vec<ReturnCode>) -> Self {
        SubAck {
            packet_id,
            return_codes,
        }
    }

    pub fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(2 + self.return_codes.len());

        buf.put_u16(self.packet_id);
        if version == MqttProtocolVersion::V5 {
            buf.put_u8(0);
            //buf.put_u8(self.properties.len() as u8);
            //for prop in self.properties {
            //buf.put(prop.into_bytes());
            //}
        }

        for code in self.return_codes {
            buf.put_u8(code.code());
        }
        buf.freeze()
    }
}

impl UnsubAck {
    pub fn new(packet_id: u16, return_codes: Vec<ReturnCode>) -> Self {
        UnsubAck {
            packet_id,
            return_codes,
        }
    }

    pub fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(2 + self.return_codes.len());

        buf.put_u16(self.packet_id);
        if version == MqttProtocolVersion::V5 {
            buf.put_u8(0);
            //let mut prop_bytes = BytesMut::new();
            //for prop in self.properties {
            //prop_bytes.put(prop.into_bytes());
            //}

            //let prop_len = prop_bytes.len();
            //if prop_len < 128 {
            //buf.put_u8(prop_len as u8);
            //} else if prop_len < 16384 {
            //buf.put_u8(((prop_len % 128) as u8) | 0x80);
            //buf.put_u8((prop_len / 128) as u8);
            //} else if prop_len < 2097152 {
            //buf.put_u8(((prop_len % 128) as u8) | 0x80);
            //buf.put_u8((((prop_len / 128) % 128) as u8) | 0x80);
            //buf.put_u8((prop_len / 16384) as u8);
            //} else {
            //buf.put_u8(((prop_len % 128) as u8) | 0x80);
            //buf.put_u8((((prop_len / 128) % 128) as u8) | 0x80);
            //buf.put_u8((((prop_len / 16384) % 128) as u8) | 0x80);
            //buf.put_u8((prop_len / 2097152) as u8);
            //}
            //buf.put(prop_bytes);
        }

        for code in self.return_codes {
            buf.put_u8(code.code());
        }
        buf.freeze()
    }
}

impl Subscribe {
    pub fn subscribe_try_from(
        rdr: &mut Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        let packet_id = rdr.read_u16::<BigEndian>()?;

        let mut subscription_identifier = None;
        if version == MqttProtocolVersion::V5 {
            let properties = Property::try_from_properties(rdr)?;
            for prop in properties {
                if let Property::SubscriptionIdentifier(id) = prop {
                    subscription_identifier = Some(id);
                }
            }
        };

        let mut topics = vec![];
        while (rdr.position() as usize) < rdr.get_ref().len() {
            let topic_len = rdr.read_u16::<BigEndian>()?;
            let mut topic = vec![0u8; topic_len as usize];

            rdr.read_exact(&mut topic)?;
            let topic = String::from_utf8_lossy(&topic).into_owned();

            let options = rdr.read_u8()?;
            let options = if MqttProtocolVersion::V5 == version {
                SubscribeOption {
                    qos: super::super::QoS::try_from(options & 0x03)?,
                    no_local: (options & 0x04) >> 2 == 1,
                    retain_as_published: (options & 0x08) >> 3 == 1,
                    retain_handling: (options & 0x30) >> 4,
                    subscription_identifier,
                }
            } else {
                SubscribeOption {
                    qos: super::super::QoS::try_from(options & 0x03)?,
                    no_local: false,
                    retain_as_published: false,
                    retain_handling: 0,
                    subscription_identifier,
                }
            };

            if options.retain_handling > 2 {
                return Err(MqttProtocolError::MalformedPayload);
            }

            topics.push((topic, options));
        }

        Ok(Message::Subscribe(Subscribe { packet_id, topics }))
    }
}

impl Unsubscribe {
    pub fn unsubscribe_try_from(
        rdr: &mut Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        let packet_id = rdr.read_u16::<BigEndian>()?;

        if version == MqttProtocolVersion::V5 {
            let _ = Property::try_from_properties(rdr)?;
        };

        let mut topics = vec![];
        while (rdr.position() as usize) < rdr.get_ref().len() {
            let topic_len = rdr.read_u16::<BigEndian>()?;
            let mut topic = vec![0u8; topic_len as usize];

            rdr.read_exact(&mut topic)?;
            let topic = String::from_utf8_lossy(&topic).into_owned();

            topics.push(topic);
        }

        Ok(Message::Unsubscribe(Unsubscribe { packet_id, topics }))
    }
}

impl std::fmt::Display for SubscribeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "options: {{ qos: {}, no_local: {}, retain_as_published: {}, retain_handling: {} }}",
            self.qos, self.no_local, self.retain_as_published, self.retain_handling
        )
    }
}
