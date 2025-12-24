use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::CONFIG;

use super::super::{MqttProtocolVersion, error::MqttProtocolError};
use super::{
    fixed::{FixedHeaderCodec, FixedOptions},
    message::Message,
};

pub struct MessageCodec {
    fixed_codec: FixedHeaderCodec,
    version: MqttProtocolVersion,
    packet_maximum: u32,
}

impl Default for MessageCodec {
    fn default() -> Self {
        MessageCodec {
            fixed_codec: FixedHeaderCodec::default(),
            version: MqttProtocolVersion::V3_1_1,
            packet_maximum: CONFIG.get().unwrap().mqtt.settings.max_packet_size,
        }
    }
}

impl MessageCodec {
    pub fn with_v5(&mut self) {
        self.version = MqttProtocolVersion::V5;
    }

    pub fn with_packet_size(&mut self, size: u32) {
        self.packet_maximum = size;
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = MqttProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let fixed_header = self.fixed_codec.decode(src)?;
        if fixed_header.is_none() {
            return Ok(None);
        }
        let fixed_header = fixed_header.unwrap();
        if fixed_header.bytes.len() as u32 + 2 > self.packet_maximum {
            return Ok(Some(Message::PacketTooLarge));
        }

        let msg = Message::try_from((fixed_header, self.version))?;
        Ok(Some(msg))
    }
}

impl Encoder<Message> for MessageCodec {
    type Error = MqttProtocolError;

    fn encode(&mut self, msg: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let options: FixedOptions = msg.into(self.version);
        let bytes: Bytes = options.into();

        dst.put(bytes);
        Ok(())
    }
}
