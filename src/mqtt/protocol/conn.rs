use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt as _};
use bytes::{BufMut, Bytes, BytesMut};
use tracing::debug;

use crate::CONFIG;

use super::super::{MqttProtocolVersion, code::ReturnCode, error::MqttProtocolError, utils};
use super::{message::Message, property::Property, will::Will};

#[derive(Clone)]
pub struct ConnectOptions {
    pub(crate) session_expiry_interval: u32,
    pub(crate) packet_maximum: u32,
    pub(crate) topic_alias_maximum: u16,
    pub(crate) inflight_maximum: u16,
}

impl std::default::Default for ConnectOptions {
    fn default() -> Self {
        let config = CONFIG.get().unwrap();
        ConnectOptions {
            session_expiry_interval: 0,
            inflight_maximum: config.mqtt.settings.max_receive_queue,
            packet_maximum: config.mqtt.settings.max_packet_size,
            topic_alias_maximum: 0,
        }
    }
}

#[derive(Clone)]
pub struct Connect {
    pub(crate) version: MqttProtocolVersion,
    pub(crate) keep_alive: u16,

    pub(crate) generate_client_id: bool,
    pub(crate) clean_start: bool,

    pub(crate) client_id: String,

    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,

    pub(crate) will: Option<Will>,
    pub(crate) options: ConnectOptions,
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

        let mut options = ConnectOptions::default();
        for prop in properties.into_iter() {
            match prop {
                Property::SessionExpiryInterval(v) => {
                    if v <= CONFIG.get().unwrap().mqtt.settings.session_expiry_interval {
                        options.session_expiry_interval = v;
                    }
                }
                Property::MaximumPacketSize(v) => {
                    if options.packet_maximum == 0 || v < options.packet_maximum {
                        options.packet_maximum = v;
                    }
                }
                Property::ReceiveMaximum(v) => {
                    if options.inflight_maximum == 0 || v < options.inflight_maximum {
                        options.inflight_maximum = v;
                    }
                }
                Property::TopicAliasMaximum(v) => {
                    if v <= CONFIG.get().unwrap().mqtt.settings.topic_alias_maximum {
                        options.topic_alias_maximum = v;
                    } else {
                        options.topic_alias_maximum =
                            CONFIG.get().unwrap().mqtt.settings.topic_alias_maximum;
                    }
                }
                _ => {
                    debug!("ignore property in CONNECT: {}", prop);
                }
            }
        }

        let client_id_len = rdr.read_u16::<BigEndian>()? as usize;
        let mut client_id_buf = vec![0; client_id_len];
        let _ = rdr.read_exact(&mut client_id_buf)?;
        let client_id = String::from_utf8_lossy(&client_id_buf).into_owned();

        //let mut will_delay_interval = None;
        //let mut will_expiry_interval = None;

        let mut will_options = super::will::WillOptions::default();
        let mut will_user_properties = Vec::new();

        if proto_version == MqttProtocolVersion::V5 && will_flag {
            let will_properties = Property::try_from_properties(rdr)?;

            for prop in will_properties.into_iter() {
                match prop {
                    Property::WillDelayInterval(v) => {
                        will_options.will_delay_interval = Some(v);
                    }
                    Property::MessageExpiryInterval(v) => {
                        will_options.options = will_options.options.with_expiry(Some(v));
                    }
                    Property::TopicAlias(v) => {
                        will_options.options.topic_alias = Some(v);
                    }
                    Property::PayloadFormatIndicator(v) => {
                        will_options.options.payload_format_indicator = Some(v);
                    }
                    Property::ContentType(v) => {
                        will_options.options.content_type = Some(v);
                    }
                    Property::ResponseTopic(v) => {
                        will_options.options.response_topic = Some(v);
                    }
                    Property::CorrelationData(v) => {
                        will_options.options.correlation_data = Some(Bytes::from(v));
                    }
                    Property::UserProperty(v) => {
                        will_user_properties.push(v);
                    }
                    _ => {
                        debug!("ignore property in CONNECT WILL: {}", prop);
                    }
                }
            }
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
                user_properties: will_user_properties,
                options: will_options,
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
            generate_client_id,
            options,
        }))
    }
}

#[derive(Clone)]
pub struct Disconnect {
    pub(crate) reason: ReturnCode,
    pub(crate) session_expiry_interval: Option<u32>,
}

impl Disconnect {
    pub(crate) fn new(reason: ReturnCode) -> Self {
        Disconnect {
            reason,
            session_expiry_interval: None,
        }
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
                session_expiry_interval: None,
            }));
        }

        let reason = rdr.read_u8()?;
        let reason = ReturnCode::try_from(reason)?;
        let properties = Property::try_from_properties(rdr)?;
        let mut session_expiry_interval = None;

        for prop in properties.into_iter() {
            match prop {
                Property::SessionExpiryInterval(v) => {
                    if v <= CONFIG.get().unwrap().mqtt.settings.session_expiry_interval {
                        session_expiry_interval = Some(v);
                    } else {
                        session_expiry_interval =
                            Some(CONFIG.get().unwrap().mqtt.settings.session_expiry_interval);
                    }
                }
                _ => {
                    debug!("ignore property in DISCONNECT: {}", prop);
                }
            }
        }

        Ok(Message::Disconnect(Disconnect {
            reason,
            session_expiry_interval,
        }))
    }
}

#[derive(Clone)]
pub struct ConnAckOptions {
    pub(crate) topic_alias_maximum: u16,
    pub(crate) session_expiry_interval: u32,
}

impl ConnAckOptions {
    fn with_topic_alias_maximum(mut self, v: u16) -> Self {
        self.topic_alias_maximum = v;
        self
    }

    fn with_session_expiry_interval(mut self, v: u32) -> Self {
        self.session_expiry_interval = v;
        self
    }
}

impl std::default::Default for ConnAckOptions {
    fn default() -> Self {
        ConnAckOptions {
            topic_alias_maximum: 0,
            session_expiry_interval: 0,
        }
    }
}

#[derive(Clone)]
pub struct ConnAck {
    session_present: bool,
    pub(crate) return_code: ReturnCode,
    generated_client_id: Option<String>,
    pub(crate) options: ConnAckOptions,
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
            options: ConnAckOptions::default(),
        }
    }

    pub fn with_topic_alias_maximum(mut self, v: u16) -> Self {
        self.options = self.options.with_topic_alias_maximum(v);
        self
    }

    pub fn with_session_expiry_interval(mut self, v: u32) -> Self {
        self.options = self.options.with_session_expiry_interval(v);
        self
    }

    pub(crate) fn into(self, version: MqttProtocolVersion) -> Bytes {
        let mut buf = BytesMut::with_capacity(2);

        let sp = if self.session_present { 0x01 } else { 0x00 };
        buf.put_u8(sp);
        buf.put_u8(self.return_code.code());

        let config = CONFIG.get().unwrap();
        if version == MqttProtocolVersion::V5 {
            let mut properties = vec![
                Property::SessionExpiryInterval(self.options.session_expiry_interval),
                Property::ServerKeepAlive(config.mqtt.settings.keep_alive),
                Property::ReceiveMaximum(config.mqtt.settings.max_receive_queue),
                Property::TopicAliasMaximum(self.options.topic_alias_maximum),
                Property::RetainAvailable(1),
                Property::WildcardSubscriptionAvailable(1),
                Property::SubscriptionIdentifierAvailable(1),
                Property::SharedSubscriptionAvailable(1),
                Property::MaximumPacketSize(config.mqtt.settings.max_packet_size),
                Property::MaximumQoS(2),
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

        buf.freeze()
    }
}
