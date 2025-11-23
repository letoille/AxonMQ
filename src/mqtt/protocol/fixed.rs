use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::Decoder;

use super::super::{QoS, error::MqttProtocolError};
use super::message::MessageType;

pub(crate) struct FixedHeaderCodec;

impl std::default::Default for FixedHeaderCodec {
    fn default() -> Self {
        FixedHeaderCodec {}
    }
}

#[derive(Debug)]
pub(crate) struct FixedOptions {
    pub(crate) dup: bool,
    pub(crate) qos: QoS,
    pub(crate) retain: bool,
    pub(crate) msg_type: MessageType,
    pub(crate) bytes: Bytes,
}

impl From<FixedOptions> for Bytes {
    fn from(options: FixedOptions) -> Self {
        let mut buf = BytesMut::with_capacity(2 + options.bytes.len());
        let mut byte1 = (options.msg_type as u8) << 4;
        match options.msg_type {
            MessageType::Subscribe | MessageType::Unsubscribe => {
                byte1 |= 0x02;
            }
            MessageType::PubRel => {
                byte1 |= 0x02;
            }
            MessageType::Publish => {
                if options.dup {
                    byte1 |= 0x08;
                }
                byte1 |= (options.qos as u8) << 1;
                if options.retain {
                    byte1 |= 0x01;
                }
            }
            _ => {}
        }
        buf.put_u8(byte1);

        let remaining = options.bytes.len();
        if remaining < 128 {
            buf.put_u8(remaining as u8);
        } else if remaining < 16384 {
            buf.put_u8(((remaining % 128) as u8) | 0x80);
            buf.put_u8((remaining / 128) as u8);
        } else if remaining < 2097152 {
            buf.put_u8(((remaining % 128) as u8) | 0x80);
            buf.put_u8((((remaining / 128) % 128) as u8) | 0x80);
            buf.put_u8((remaining / 16384) as u8);
        } else {
            buf.put_u8(((remaining % 128) as u8) | 0x80);
            buf.put_u8((((remaining / 128) % 128) as u8) | 0x80);
            buf.put_u8((((remaining / 16384) % 128) as u8) | 0x80);
            buf.put_u8((remaining / 2097152) as u8);
        }

        buf.put(options.bytes);
        buf.freeze()
    }
}

impl Decoder for FixedHeaderCodec {
    type Item = FixedOptions;
    type Error = MqttProtocolError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 2 {
            return Ok(None);
        }
        let msg_type = MessageType::try_from((src[0] & 0xF0) >> 4)?;

        let mut dup = false;
        let mut qos = QoS::AtMostOnce;
        let mut retain = false;

        match msg_type {
            MessageType::Subscribe | MessageType::Unsubscribe => {
                if src[0] & 0x0F != 2 {
                    return Err(MqttProtocolError::InvalidFixedHeader);
                }
            }
            MessageType::Publish => {
                let q = (src[0] & 0x06) >> 1;
                qos = QoS::try_from(q)?;
                dup = (src[0] & 0x08) >> 3 == 1;
                retain = (src[0] & 0x01) == 1;
            }
            MessageType::PubRel => {
                if src[0] & 0x0F != 2 {
                    return Err(MqttProtocolError::InvalidFixedHeader);
                }
            }
            _ => {
                if src[0] & 0x0F != 0 {
                    return Err(MqttProtocolError::InvalidFixedHeader);
                }
            }
        }

        #[allow(unused_assignments)]
        let mut length = 0;
        let mut length_bytes = 1;
        if src[1] & 0x80 > 0 {
            if src.len() < 3 {
                return Ok(None);
            }
            if src[2] & 0x80 > 0 {
                if src.len() < 4 {
                    return Ok(None);
                }
                if src[3] & 0x80 > 0 {
                    if src.len() < 5 {
                        return Ok(None);
                    }
                    length = ((src[1] & 0x7F) as u32)
                        | (((src[2] & 0x7F) as u32) << 7)
                        | (((src[3] & 0x7F) as u32) << 14)
                        | (((src[4] & 0x7F) as u32) << 21);
                    length_bytes = 4;
                } else {
                    length_bytes = 3;
                    length = ((src[1] & 0x7F) as u32)
                        | (((src[2] & 0x7F) as u32) << 7)
                        | (((src[3] & 0x7F) as u32) << 14);
                }
            } else {
                length_bytes = 2;
                length = ((src[1] & 0x7F) as u32) | (((src[2] & 0x7F) as u32) << 7);
            }
        } else {
            length = (src[1] & 0x7F) as u32;
        }

        if src.len() < (length as usize + 1 + length_bytes) {
            return Ok(None);
        }

        let _fix_header = src.split_to(1 + length_bytes);
        let options = FixedOptions {
            dup,
            qos,
            retain,
            msg_type,
            bytes: src.split_to(length as usize).freeze(),
        };
        Ok(Some(options))
    }
}
