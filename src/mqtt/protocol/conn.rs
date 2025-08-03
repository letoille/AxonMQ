use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt as _};
use bytes::{BufMut, Bytes, BytesMut};
use tracing::debug;

use crate::CONFIG;

use super::super::{MqttProtocolVersion, code::ReturnCode, error::MqttProtocolError, utils};
use super::{message::Message, property::Property, will::Will};

#[derive(Clone)]
pub struct Connect {
    pub(crate) version: MqttProtocolVersion,
    pub(crate) keep_alive: u16,

    pub(crate) client_id: String,
    pub(crate) generate_client_id: bool,

    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,

    pub(crate) clean_start: bool,

    pub(crate) will: Option<Will>,

    pub(crate) session_expiry_interval: Option<u32>,
    pub(crate) inflight_maximum: Option<u16>,
    pub(crate) packet_maximum: Option<u32>,
}

impl From<Connect> for Bytes {
    fn from(conn: Connect) -> Self {
        let mut buf = BytesMut::with_capacity(32);

        buf.put_u16(4);
        buf.put_slice(b"MQTT");

        buf.put_u8(conn.version.code());

        let mut flag = 0;
        if conn.username.is_some() {
            flag |= 0b1000_0000;
        }
        if conn.password.is_some() {
            flag |= 0b0100_0000;
        }
        if let Some(will) = &conn.will {
            flag |= 0b0000_0100;
            if will.retain {
                flag |= 0b0010_0000;
            }
            flag |= (will.qos as u8) << 3;
        }
        if conn.clean_start {
            flag |= 0b0000_0010;
        }

        buf.put_u8(flag);
        buf.put_u16(conn.keep_alive);

        todo!("properties");

        //buf.freeze()
    }
}

impl Connect {
    pub(crate) fn connect_try_from(rdr: &mut Cursor<Bytes>) -> Result<Message, MqttProtocolError> {
        let proto_name_len = rdr.read_u16::<BigEndian>()?;
        if proto_name_len != 4 {
            return Err(MqttProtocolError::InvalidProtocolName);
        }

        let proto_name = rdr.read_u32::<BigEndian>()?;
        if proto_name != 0x4d515454 {
            return Err(MqttProtocolError::InvalidProtocolName);
        }

        let proto_version = rdr.read_u8()?;
        let proto_version = MqttProtocolVersion::try_from(proto_version)?;

        let flag = rdr.read_u8()?;
        let user_flag = (flag & 0x80) >> 7 == 1;
        let pass_flag = (flag & 0x40) >> 6 == 1;
        let will_retain = (flag & 0x20) >> 5 == 1;
        let will_qos = (flag & 0x18) >> 3;
        let will_qos = super::super::QoS::try_from(will_qos)?;
        let will_flag = (flag & 0x04) >> 2 == 1;
        let mut clean_session = (flag & 0x02) >> 1 == 1;

        let keep_alive = rdr.read_u16::<BigEndian>()?;
        let mut properties = Vec::new();
        if proto_version == MqttProtocolVersion::V5 {
            properties = Property::try_from_properties(rdr)?;
        }

        let mut session_expiry_interval = None;
        let mut inflight_maximum = None;
        let mut packet_maximum = None;
        for prop in properties.iter() {
            match prop {
                Property::SessionExpiryInterval(v) => session_expiry_interval = Some(*v),
                Property::MaximumPacketSize(v) => packet_maximum = Some(*v),
                Property::ReceiveMaximum(v) => inflight_maximum = Some(*v),
                _ => {
                    debug!("ignore property in CONNECT: {}", prop);
                }
            }
        }
        properties.retain(|p| {
            !matches!(
                p,
                Property::SessionExpiryInterval(_)
                    | Property::MaximumPacketSize(_)
                    | Property::ReceiveMaximum(_)
            )
        });

        let client_id_len = rdr.read_u16::<BigEndian>()? as usize;
        let mut client_id_buf = vec![0; client_id_len];
        let _ = rdr.read_exact(&mut client_id_buf)?;
        let client_id = String::from_utf8_lossy(&client_id_buf).into_owned();

        let mut will_properties = Vec::new();
        let mut will_delay_interval = None;
        let mut will_expiry_interval = None;

        if proto_version == MqttProtocolVersion::V5 && will_flag {
            will_properties = Property::try_from_properties(rdr)?;

            for prop in will_properties.iter() {
                match prop {
                    Property::WillDelayInterval(v) => will_delay_interval = Some(*v),
                    Property::MessageExpiryInterval(v) => will_expiry_interval = Some(*v as u64),
                    _ => {
                        debug!("ignore property in CONNECT WILL: {}", prop);
                    }
                }
            }
            will_properties.retain(|p| {
                !matches!(
                    p,
                    Property::WillDelayInterval(_) | Property::MessageExpiryInterval(_)
                )
            });
        }

        let will = if will_flag {
            let will_topic_len = rdr.read_u16::<BigEndian>()? as usize;
            let mut will_topic_buf = vec![0; will_topic_len];
            let _ = rdr.read_exact(&mut will_topic_buf)?;
            let will_topic = String::from_utf8_lossy(&will_topic_buf).into_owned();

            let will_message_len = rdr.read_u16::<BigEndian>()? as usize;
            let end_offset = rdr.position() as usize;
            let will_msg = rdr
                .get_ref()
                .slice(end_offset..end_offset + will_message_len);
            rdr.set_position((end_offset + will_message_len) as u64);

            Some(Will {
                retain: will_retain,
                qos: will_qos,
                topic: will_topic,
                payload: will_msg,
                properties: will_properties,
                will_delay_interval: will_delay_interval.unwrap_or(0),
                expiry_interval: will_expiry_interval,
            })
        } else {
            None
        };

        let username = if user_flag {
            let username_len = rdr.read_u16::<BigEndian>()? as usize;
            let mut username_buf = vec![0; username_len];
            let _ = rdr.read_exact(&mut username_buf)?;
            Some(String::from_utf8_lossy(&username_buf).into_owned())
        } else {
            None
        };

        let password = if pass_flag {
            let password_len = rdr.read_u16::<BigEndian>()? as usize;
            let mut password_buf = vec![0; password_len];
            let _ = rdr.read_exact(&mut password_buf)?;
            Some(String::from_utf8_lossy(&password_buf).into_owned())
        } else {
            None
        };

        let mut generate_client_id = false;
        let client_id = if client_id.len() == 0 {
            clean_session = true;
            session_expiry_interval = Some(0);
            generate_client_id = true;
            utils::generate_random_client_id()
        } else {
            client_id
        };

        Ok(Message::Connect(Connect {
            version: proto_version,
            keep_alive,
            clean_start: clean_session,
            client_id,
            username,
            password,
            will,
            session_expiry_interval,
            inflight_maximum,
            packet_maximum,
            generate_client_id,
        }))
    }
}

#[derive(Clone)]
pub struct Disconnect {
    pub(crate) reason: ReturnCode,
}

impl Disconnect {
    pub(crate) fn new(reason: ReturnCode) -> Self {
        Disconnect { reason }
    }

    pub(crate) fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::new();
        if version == MqttProtocolVersion::V3_1_1 || version == MqttProtocolVersion::V3 {
            return buf.freeze();
        }

        buf.put_u8(self.reason.code());
        buf.freeze()
    }

    pub fn disconnect_try_from(
        rdr: &mut Cursor<Bytes>,
        version: MqttProtocolVersion,
    ) -> Result<Message, MqttProtocolError> {
        if version == MqttProtocolVersion::V3_1_1 || version == MqttProtocolVersion::V3 {
            return Ok(Message::Disconnect(Disconnect {
                reason: ReturnCode::Success,
            }));
        }

        let reason = rdr.read_u8()?;
        let reason = ReturnCode::try_from(reason)?;

        Ok(Message::Disconnect(Disconnect { reason }))
    }
}

#[derive(Clone)]
pub struct ConnAck {
    session_present: bool,
    pub(crate) return_code: ReturnCode,
    generated_client_id: Option<String>,
}

impl ConnAck {
    pub(crate) fn new(
        session_present: bool,
        return_code: ReturnCode,
        generated_client_id: Option<String>,
    ) -> Self {
        ConnAck {
            session_present,
            return_code,
            generated_client_id,
        }
    }

    pub(crate) fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(2);

        let sp = if self.session_present { 0x01 } else { 0x00 };
        buf.put_u8(sp);
        buf.put_u8(self.return_code.code());

        let config = CONFIG.get().unwrap();
        if version == MqttProtocolVersion::V5 {
            let mut properties = vec![
                Property::SessionExpiryInterval(config.mqtt.settings.session_expiry_interval),
                Property::ServerKeepAlive(config.mqtt.settings.keep_alive),
                Property::ReceiveMaximum(config.mqtt.settings.max_receive_queue),
                Property::TopicAliasMaximum(0),
                Property::RetainAvailable(1),
                Property::WildcardSubscriptionAvailable(1),
                Property::SubscriptionIdentifierAvailable(1),
                Property::SharedSubscriptionAvailable(1),
                Property::MaximumPacketSize(config.mqtt.settings.max_packet_size),
            ];
            if let Some(client_id) = self.generated_client_id {
                properties.push(Property::AssignedClientIdentifier(client_id));
            }

            let mut prop_bytes = BytesMut::new();
            for prop in properties {
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

        buf.freeze()
    }
}
