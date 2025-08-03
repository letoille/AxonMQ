use bytes::Bytes;

use super::super::{MqttProtocolVersion, QoS, error::MqttProtocolError};
use super::{conn, fixed::FixedOptions, publish, subscribe};

#[derive(Debug, Clone, Copy)]
pub(crate) enum MessageType {
    #[allow(dead_code)]
    Reserved0 = 0,
    Connect = 1,
    ConnAck = 2,
    Publish = 3,
    PubAck = 4,
    PubRec = 5,
    PubRel = 6,
    PubComp = 7,
    Subscribe = 8,
    SubAck = 9,
    Unsubscribe = 10,
    UnsubAck = 11,
    PingReq = 12,
    PingResp = 13,
    Disconnect = 14,
    Auth = 15,
}

impl TryFrom<u8> for MessageType {
    type Error = MqttProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Connect),
            2 => Ok(MessageType::ConnAck),
            3 => Ok(MessageType::Publish),
            4 => Ok(MessageType::PubAck),
            5 => Ok(MessageType::PubRec),
            6 => Ok(MessageType::PubRel),
            7 => Ok(MessageType::PubComp),
            8 => Ok(MessageType::Subscribe),
            9 => Ok(MessageType::SubAck),
            10 => Ok(MessageType::Unsubscribe),
            11 => Ok(MessageType::UnsubAck),
            12 => Ok(MessageType::PingReq),
            13 => Ok(MessageType::PingResp),
            14 => Ok(MessageType::Disconnect),
            15 => Ok(MessageType::Auth),
            _ => Err(MqttProtocolError::InvalidMessageType),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum Message {
    Connect(conn::Connect),
    ConnAck(conn::ConnAck),
    Publish(publish::Publish),
    PubAck(publish::PubAck),
    PubRec(publish::PubRec),
    PubRel(publish::PubRel),
    PubComp(publish::PubComp),
    Subscribe(subscribe::Subscribe),
    SubAck(subscribe::SubAck),
    Unsubscribe(subscribe::Unsubscribe),
    UnsubAck(subscribe::UnsubAck),
    PingReq,
    PingResp,
    Disconnect(conn::Disconnect),
    Auth,
    PacketTooLarge,
}

impl Message {
    pub fn with_dup(&mut self) {
        if let Message::Publish(publish) = self {
            publish.dup = true;
        }
    }

    pub fn to_type(&self) -> MessageType {
        match self {
            Message::Connect(_) => MessageType::Connect,
            Message::ConnAck(_) => MessageType::ConnAck,
            Message::Publish(_) => MessageType::Publish,
            Message::PubAck(_) => MessageType::PubAck,
            Message::PubRec(_) => MessageType::PubRec,
            Message::PubRel(_) => MessageType::PubRel,
            Message::PubComp(_) => MessageType::PubComp,
            Message::Subscribe(_) => MessageType::Subscribe,
            Message::SubAck(_) => MessageType::SubAck,
            Message::Unsubscribe(_) => MessageType::Unsubscribe,
            Message::UnsubAck(_) => MessageType::UnsubAck,
            Message::PingReq => MessageType::PingReq,
            Message::PingResp => MessageType::PingResp,
            Message::Disconnect(_) => MessageType::Disconnect,
            Message::Auth => MessageType::Auth,
            Message::PacketTooLarge => MessageType::Reserved0,
        }
    }

    pub fn into(self, version: MqttProtocolVersion) -> FixedOptions {
        let msg_type = self.to_type();
        let mut retain = false;
        let mut dup = false;
        let mut qos = QoS::AtMostOnce;

        let bytes = match self {
            Message::Publish(publish) => {
                retain = publish.retain;
                dup = publish.dup;
                qos = publish.qos;
                publish.into(version)
            }
            Message::Connect(conn) => conn.into(),
            Message::ConnAck(connack) => connack.into(version),
            Message::SubAck(suback) => suback.into(version),
            Message::UnsubAck(unsuback) => unsuback.into(version),
            Message::Disconnect(disconnect) => disconnect.into(version),
            Message::Subscribe(_subscribe) => todo!("subscribe.into(version)"),
            Message::Unsubscribe(_unsubscribe) => todo!("unsubscribe.into(version)"),
            Message::PubAck(puback) => puback.into(version),
            Message::PubRec(pubrec) => pubrec.into(version),
            Message::PubRel(pubrel) => pubrel.into(version),
            Message::PubComp(pubcomp) => pubcomp.into(version),
            Message::PingReq => Bytes::new(),
            Message::PingResp => Bytes::new(),
            _ => Bytes::new(),
        };

        FixedOptions {
            qos,
            dup,
            retain,
            msg_type: msg_type,
            bytes,
        }
    }
}

impl TryFrom<(FixedOptions, MqttProtocolVersion)> for Message {
    type Error = MqttProtocolError;

    fn try_from(value: (FixedOptions, MqttProtocolVersion)) -> Result<Self, Self::Error> {
        let (fixed_options, protocol_version) = value;
        let mut rdr = std::io::Cursor::new(fixed_options.bytes);
        match fixed_options.msg_type {
            MessageType::Connect => conn::Connect::connect_try_from(&mut rdr),
            MessageType::PingReq => Ok(Message::PingReq),
            MessageType::Subscribe => {
                subscribe::Subscribe::subscribe_try_from(&mut rdr, protocol_version)
            }
            MessageType::Unsubscribe => {
                subscribe::Unsubscribe::unsubscribe_try_from(&mut rdr, protocol_version)
            }
            MessageType::Publish => publish::Publish::publish_try_from(
                &mut rdr,
                protocol_version,
                fixed_options.qos,
                fixed_options.dup,
                fixed_options.retain,
            ),
            MessageType::Disconnect => {
                conn::Disconnect::disconnect_try_from(&mut rdr, protocol_version)
            }
            MessageType::PubAck => publish::PubAck::puback_try_from(&mut rdr, protocol_version),
            MessageType::PubRec => publish::PubRec::pubrec_try_from(&mut rdr, protocol_version),
            MessageType::PubRel => publish::PubRel::pubrel_try_from(&mut rdr, protocol_version),
            MessageType::PubComp => publish::PubComp::pubcomp_try_from(&mut rdr, protocol_version),
            _ => Err(MqttProtocolError::InvalidMessageType),
        }
    }
}
