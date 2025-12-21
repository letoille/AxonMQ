// mime type
/*
 text/plain
 text/html
 text/csv
 application/json
 application/xml
 application/x-www-form-urlencoded
 application/graphql

 application/octet-stream
 application/pdf
 application/zip
 application/protobuf   protocol buffers
 application/flatbuffers
 application/cbor       concise binary object representation
 image/jpeg
 image/png
 audio/mpeg
 video/mp4
*/

use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt as _};
use bytes::{BufMut, Bytes, BytesMut};

use super::super::error::MqttProtocolError;

#[derive(Clone)]
pub struct PropertyUser {
    pub key: String,
    pub value: String,
}

impl std::default::Default for PropertyUser {
    fn default() -> Self {
        PropertyUser {
            key: String::new(),
            value: String::new(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum Property {
    ReasonString(String), // detail reason. connack, puback, pubrec, pubrel, pubcomp, suback, unsuback, disconnect, auth
    ServerReference(String), // if broker reject connection, it can use this property to inform client to connect to another server. connack, disconnect
    ResponseInformation(String), // broker-specific response information. connack
    RequestResponseInformation(u8), // if client set to 1, server should send response information. connect
    RequestProblemInformation(u8), // if client set to 0, server should not send problem information. connect
    AuthenticationMethod(String),  // connect, connack, auth
    AuthenticationData(Vec<u8>),   // connect, connack, auth

    ResponseTopic(String),    // response topic. publish, subscribe properties
    CorrelationData(Vec<u8>), // used to correlate request/response. publish, subscribe properties

    PayloadFormatIndicator(u8), // 0: binary, 1: utf8. publish, will properties
    ContentType(String),        // mime type of payload. publish, will properties

    ServerKeepAlive(u16), //  server actually keep alive (second). connack

    AssignedClientIdentifier(String),    // connack
    SubscriptionIdentifier(u32),         // publish, subscribe properties
    SessionExpiryInterval(u32), // session stay in broker (second). connect, connack, disconnect
    MessageExpiryInterval(u32), // message stay in broker (second). publish, will properties
    WillDelayInterval(u32),     // will delay interval (second). will properties
    ReceiveMaximum(u16), // maximum number of QoS 1 and QoS 2 publications that the server is willing to process concurrently. connect, connack
    TopicAliasMaximum(u16), // maximum number of topic aliases that the server is willing to accept. connect, connack
    TopicAlias(u16),        // topic alias. publish
    MaximumQoS(u8),         // maximum QoS that the server is willing to accept. connack
    RetainAvailable(u8),    // if 1, server accept retained messages.  connack
    MaximumPacketSize(u32), // client/server maximum packet size. connect, connack
    WildcardSubscriptionAvailable(u8), // if 1, server accept subscription with wildcard. connack
    SubscriptionIdentifierAvailable(u8), // if 1, server accept subscription with subscription identifier. connack
    SharedSubscriptionAvailable(u8), // if 1, server accept subscription with shared subscription.

    UserProperty(PropertyUser), // user property. all packets except pubrel
}

impl std::fmt::Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Property::PayloadFormatIndicator(v) => write!(f, "PayloadFormatIndicator: {}", v),
            Property::MessageExpiryInterval(v) => write!(f, "MessageExpiryInterval: {}", v),
            Property::ContentType(v) => write!(f, "ContentType: {}", v),
            Property::ResponseTopic(v) => write!(f, "ResponseTopic: {}", v),
            Property::CorrelationData(v) => write!(f, "CorrelationData: {:?}", v),
            Property::SubscriptionIdentifier(v) => write!(f, "SubscriptionIdentifier: {}", v),
            Property::SessionExpiryInterval(v) => write!(f, "SessionExpiryInterval: {}", v),
            Property::AssignedClientIdentifier(v) => write!(f, "AssignedClientIdentifier: {}", v),
            Property::ServerKeepAlive(v) => write!(f, "ServerKeepAlive: {}", v),
            Property::AuthenticationMethod(v) => write!(f, "AuthenticationMethod: {}", v),
            Property::AuthenticationData(v) => write!(f, "AuthenticationData: {:?}", v),
            Property::RequestProblemInformation(v) => {
                write!(f, "RequestProblemInformation: {}", v)
            }
            Property::WillDelayInterval(v) => write!(f, "WillDelayInterval: {}", v),
            Property::RequestResponseInformation(v) => {
                write!(f, "RequestResponseInformation: {}", v)
            }
            Property::ResponseInformation(v) => write!(f, "ResponseInformation: {}", v),
            Property::ServerReference(v) => write!(f, "ServerReference: {}", v),
            Property::ReasonString(v) => write!(f, "ReasonString: {}", v),
            Property::ReceiveMaximum(v) => write!(f, "ReceiveMaximum: {}", v),
            Property::TopicAliasMaximum(v) => write!(f, "TopicAliasMaximum: {}", v),
            Property::TopicAlias(v) => write!(f, "TopicAlias: {}", v),
            Property::MaximumQoS(v) => write!(f, "MaximumQoS: {}", v),
            Property::RetainAvailable(v) => write!(f, "RetainAvailable: {}", v),
            Property::UserProperty(p) => write!(f, "UserProperty: {}={}", p.key, p.value),
            Property::MaximumPacketSize(v) => write!(f, "MaximumPacketSize: {}", v),
            Property::WildcardSubscriptionAvailable(v) => {
                write!(f, "WildcardSubscriptionAvailable: {}", v)
            }
            Property::SubscriptionIdentifierAvailable(v) => {
                write!(f, "SubscriptionIdentifierAvailable: {}", v)
            }
            Property::SharedSubscriptionAvailable(v) => {
                write!(f, "SharedSubscriptionAvailable: {}", v)
            }
        }
    }
}

impl Property {
    pub(crate) fn code(&self) -> u8 {
        match self {
            Property::PayloadFormatIndicator(_) => 0x01,
            Property::MessageExpiryInterval(_) => 0x02,
            Property::ContentType(_) => 0x03,
            Property::ResponseTopic(_) => 0x08,
            Property::CorrelationData(_) => 0x09,
            Property::SubscriptionIdentifier(_) => 0x0B,
            Property::SessionExpiryInterval(_) => 0x11,
            Property::AssignedClientIdentifier(_) => 0x12,
            Property::ServerKeepAlive(_) => 0x13,
            Property::AuthenticationMethod(_) => 0x15,
            Property::AuthenticationData(_) => 0x16,
            Property::RequestProblemInformation(_) => 0x17,
            Property::WillDelayInterval(_) => 0x18,
            Property::RequestResponseInformation(_) => 0x19,
            Property::ResponseInformation(_) => 0x1A,
            Property::ServerReference(_) => 0x1C,
            Property::ReasonString(_) => 0x1F,
            Property::ReceiveMaximum(_) => 0x21,
            Property::TopicAliasMaximum(_) => 0x22,
            Property::TopicAlias(_) => 0x23,
            Property::MaximumQoS(_) => 0x24,
            Property::RetainAvailable(_) => 0x25,
            Property::UserProperty(_) => 0x26,
            Property::MaximumPacketSize(_) => 0x27,
            Property::WildcardSubscriptionAvailable(_) => 0x28,
            Property::SubscriptionIdentifierAvailable(_) => 0x29,
            Property::SharedSubscriptionAvailable(_) => 0x2A,
        }
    }

    fn try_from_code(code: u8) -> Result<Property, MqttProtocolError> {
        match code {
            0x01 => Ok(Property::PayloadFormatIndicator(0)),
            0x02 => Ok(Property::MessageExpiryInterval(0)),
            0x03 => Ok(Property::ContentType(String::new())),
            0x08 => Ok(Property::ResponseTopic(String::new())),
            0x09 => Ok(Property::CorrelationData(Vec::new())),
            0x0B => Ok(Property::SubscriptionIdentifier(0)),
            0x11 => Ok(Property::SessionExpiryInterval(0)),
            0x12 => Ok(Property::AssignedClientIdentifier(String::new())),
            0x13 => Ok(Property::ServerKeepAlive(0)),
            0x15 => Ok(Property::AuthenticationMethod(String::new())),
            0x16 => Ok(Property::AuthenticationData(Vec::new())),
            0x17 => Ok(Property::RequestProblemInformation(0)),
            0x18 => Ok(Property::WillDelayInterval(0)),
            0x19 => Ok(Property::RequestResponseInformation(0)),
            0x1A => Ok(Property::ResponseInformation(String::new())),
            0x1C => Ok(Property::ServerReference(String::new())),
            0x1F => Ok(Property::ReasonString(String::new())),
            0x21 => Ok(Property::ReceiveMaximum(0)),
            0x22 => Ok(Property::TopicAliasMaximum(0)),
            0x23 => Ok(Property::TopicAlias(0)),
            0x24 => Ok(Property::MaximumQoS(0)),
            0x25 => Ok(Property::RetainAvailable(0)),
            0x26 => Ok(Property::UserProperty(PropertyUser::default())),
            0x27 => Ok(Property::MaximumPacketSize(0)),
            0x28 => Ok(Property::WildcardSubscriptionAvailable(0)),
            0x29 => Ok(Property::SubscriptionIdentifierAvailable(0)),
            0x2A => Ok(Property::SharedSubscriptionAvailable(0)),
            _ => Err(MqttProtocolError::InvalidProperty),
        }
    }

    pub(crate) fn into_bytes(&self) -> Bytes {
        use Property::*;

        let mut buf = BytesMut::new();
        buf.put_u8(self.code());

        match self {
            PayloadFormatIndicator(v)
            | RequestProblemInformation(v)
            | RequestResponseInformation(v)
            | MaximumQoS(v)
            | RetainAvailable(v)
            | WildcardSubscriptionAvailable(v)
            | SubscriptionIdentifierAvailable(v)
            | SharedSubscriptionAvailable(v) => {
                buf.put_u8(*v);
            }
            ServerKeepAlive(v) | ReceiveMaximum(v) | TopicAliasMaximum(v) | TopicAlias(v) => {
                buf.put_u16(*v);
            }
            MessageExpiryInterval(v)
            | SessionExpiryInterval(v)
            | WillDelayInterval(v)
            | MaximumPacketSize(v) => {
                buf.put_u32(*v);
            }
            ContentType(v)
            | ResponseTopic(v)
            | AssignedClientIdentifier(v)
            | AuthenticationMethod(v)
            | ResponseInformation(v)
            | ServerReference(v)
            | ReasonString(v) => {
                buf.put_u16(v.len() as u16);
                buf.put_slice(v.as_bytes());
            }
            CorrelationData(v) | AuthenticationData(v) => {
                buf.put_u16(v.len() as u16);
                buf.put_slice(v);
            }
            SubscriptionIdentifier(v) => {
                // variable byte integer
                let mut x = *v;
                loop {
                    let mut encoded_byte = (x % 128) as u8;
                    x /= 128;
                    if x > 0 {
                        encoded_byte |= 128;
                    }
                    buf.put_u8(encoded_byte);
                    if x == 0 {
                        break;
                    }
                }
            }
            UserProperty(p) => {
                buf.put_u16(p.key.len() as u16);
                buf.put_slice(p.key.as_bytes());
                buf.put_u16(p.value.len() as u16);
                buf.put_slice(p.value.as_bytes());
            }
        }

        buf.freeze()
    }

    fn try_from_property(rdr: &mut Cursor<Bytes>) -> Result<Property, MqttProtocolError> {
        use Property::*;

        let code = rdr.read_u8()?;
        let mut property = Property::try_from_code(code)?;

        match property {
            PayloadFormatIndicator(ref mut v)
            | RequestProblemInformation(ref mut v)
            | RequestResponseInformation(ref mut v)
            | MaximumQoS(ref mut v)
            | RetainAvailable(ref mut v)
            | WildcardSubscriptionAvailable(ref mut v)
            | SubscriptionIdentifierAvailable(ref mut v)
            | SharedSubscriptionAvailable(ref mut v) => {
                *v = rdr.read_u8()?;
            }
            ServerKeepAlive(ref mut v)
            | ReceiveMaximum(ref mut v)
            | TopicAliasMaximum(ref mut v)
            | TopicAlias(ref mut v) => {
                *v = rdr.read_u16::<BigEndian>()?;
            }
            MessageExpiryInterval(ref mut v)
            | SessionExpiryInterval(ref mut v)
            | WillDelayInterval(ref mut v)
            | MaximumPacketSize(ref mut v) => {
                *v = rdr.read_u32::<BigEndian>()?;
            }
            ContentType(ref mut v)
            | ResponseTopic(ref mut v)
            | AssignedClientIdentifier(ref mut v)
            | AuthenticationMethod(ref mut v)
            | ResponseInformation(ref mut v)
            | ServerReference(ref mut v)
            | ReasonString(ref mut v) => {
                let len = rdr.read_u16::<BigEndian>()? as usize;
                let mut buf = vec![0; len];
                rdr.read_exact(&mut buf)?;
                *v = String::from_utf8(buf).map_err(|_| MqttProtocolError::InvalidProperty)?;
            }
            CorrelationData(ref mut v) | AuthenticationData(ref mut v) => {
                let len = rdr.read_u16::<BigEndian>()? as usize;
                let mut buf = vec![0; len];
                rdr.read_exact(&mut buf)?;
                *v = buf;
            }
            SubscriptionIdentifier(ref mut v) => {
                // variable byte integer
                let mut multiplier = 1;
                let mut value = 0;
                loop {
                    let encoded_byte = rdr.read_u8()?;
                    value += ((encoded_byte & 127) as u32) * multiplier;
                    if (encoded_byte & 128) == 0 {
                        break;
                    }
                    multiplier *= 128;
                    if multiplier > 128 * 128 * 128 {
                        return Err(MqttProtocolError::MalformedPayload);
                    }
                }
                *v = value;
            }
            UserProperty(ref mut v) => {
                let key_len = rdr.read_u16::<BigEndian>()? as usize;
                let mut key_buf = vec![0; key_len];
                rdr.read_exact(&mut key_buf)?;
                let key =
                    String::from_utf8(key_buf).map_err(|_| MqttProtocolError::InvalidProperty)?;

                let value_len = rdr.read_u16::<BigEndian>()? as usize;
                let mut value_buf = vec![0; value_len];
                rdr.read_exact(&mut value_buf)?;
                let value =
                    String::from_utf8(value_buf).map_err(|_| MqttProtocolError::InvalidProperty)?;

                *v = PropertyUser { key, value };
            }
        }

        Ok(property)
    }

    pub(crate) fn try_from_properties(
        rdr: &mut Cursor<Bytes>,
    ) -> Result<Vec<Property>, MqttProtocolError> {
        let len0 = rdr.read_u8()?;
        let length = if len0 & 0x80 > 0 {
            let len1 = rdr.read_u8()?;
            if len1 & 0x80 > 0 {
                let len2 = rdr.read_u8()?;
                if len2 & 0x80 > 0 {
                    let len3 = rdr.read_u8()?;
                    ((len0 & 0x7F) as u32)
                        | (((len1 & 0x7F) as u32) << 7)
                        | (((len2 & 0x7F) as u32) << 14)
                        | (((len3 & 0x7F) as u32) << 21)
                } else {
                    ((len0 & 0x7F) as u32)
                        | (((len1 & 0x7F) as u32) << 7)
                        | (((len2 & 0x7F) as u32) << 14)
                }
            } else {
                ((len0 & 0x7F) as u32) | (((len1 & 0x7F) as u32) << 7)
            }
        } else {
            (len0 & 0x7F) as u32
        };

        let start_pos = rdr.position();
        let end_pos = start_pos + length as u64;

        let mut properties = Vec::new();
        while rdr.position() < end_pos {
            let property = Property::try_from_property(rdr)?;
            properties.push(property);
        }

        Ok(properties)
    }
}
