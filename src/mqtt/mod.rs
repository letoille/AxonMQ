mod code;
pub mod command;
mod error;
pub mod helper;
pub mod protocol;
pub mod server;
mod stack;
mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl QoS {
    pub fn min(self, other: QoS) -> QoS {
        if self as u8 <= other as u8 {
            self
        } else {
            other
        }
    }
}

impl std::fmt::Display for QoS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QoS::AtMostOnce => write!(f, "AtMostOnce(0)"),
            QoS::AtLeastOnce => write!(f, "AtLeastOnce(1)"),
            QoS::ExactlyOnce => write!(f, "ExactlyOnce(2)"),
        }
    }
}

impl TryFrom<u8> for QoS {
    type Error = error::MqttProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            _ => Err(error::MqttProtocolError::InvalidQoS),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MqttProtocolVersion {
    V3 = 3,
    V3_1_1 = 4,
    V5 = 5,
}

impl MqttProtocolVersion {
    pub fn code(&self) -> u8 {
        match self {
            MqttProtocolVersion::V3 => 3,
            MqttProtocolVersion::V3_1_1 => 4,
            MqttProtocolVersion::V5 => 5,
        }
    }
}

impl std::fmt::Display for MqttProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MqttProtocolVersion::V3 => write!(f, "3.1"),
            MqttProtocolVersion::V3_1_1 => write!(f, "3.1.1"),
            MqttProtocolVersion::V5 => write!(f, "5.0"),
        }
    }
}

impl TryFrom<u8> for MqttProtocolVersion {
    type Error = error::MqttProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(MqttProtocolVersion::V3),
            4 => Ok(MqttProtocolVersion::V3_1_1),
            5 => Ok(MqttProtocolVersion::V5),
            _ => Err(error::MqttProtocolError::InvalidProtocolVersion),
        }
    }
}
