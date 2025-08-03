//https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.pdf
//https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.pdf

use super::error::MqttProtocolError;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReturnCode {
    Success = 0,                    // connection accepted
    UnsupportedProtocolVersion = 1, // connection refused, unacceptable protocol version
    IdentifierRejected = 2,         // connection refused, identifier rejected, correct utf8 but not
    ServerUnavailable = 3,          // connection refused, server unavailable
    InvalidUsernameOrPassword = 4,  // connection refused, bad user name or password
    NotAuthorized = 5,              // connection refused, not authorized
    NoMatchSubscription = 16,       // v5
    NoSubscriptionExisted = 17,     // v5
    // v3 6-255: reserved
    UnspecifiedError = 128,
    MalformedPacket = 129,
    ProtocolError = 130,
    ImSpecificError = 131,
    UnsupProtoVersion = 132,
    ClientIdNotValid = 133,
    BadUserNameOrPassword = 134,
    NotAuthorizedV5 = 135,
    ServerUnavailableV5 = 136,
    ServerBusy = 137,
    Banned = 138,
    BadAuthMethod = 140,
    SessionTakenOver = 142,
    TopicFilterInvalid = 143,
    TopicNameInvalid = 144,
    PacketIdentifierNotFound = 146,
    PacketTooLarge = 149,
    QuotaExceeded = 151,
    PayloadFormatInvalid = 153,
    RetainNotSupported = 154,
    QoSNotSupported = 155,
    UseAnotherServer = 156,
    ServerMoved = 157,
    ConnectionRateExceeded = 159,
}

impl ReturnCode {
    pub(crate) fn code(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for ReturnCode {
    type Error = MqttProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReturnCode::Success),
            1 => Ok(ReturnCode::UnsupportedProtocolVersion),
            2 => Ok(ReturnCode::IdentifierRejected),
            3 => Ok(ReturnCode::ServerUnavailable),
            4 => Ok(ReturnCode::InvalidUsernameOrPassword),
            5 => Ok(ReturnCode::NotAuthorized),
            16 => Ok(ReturnCode::NoMatchSubscription),
            17 => Ok(ReturnCode::NoSubscriptionExisted),
            128 => Ok(ReturnCode::UnspecifiedError),
            129 => Ok(ReturnCode::MalformedPacket),
            130 => Ok(ReturnCode::ProtocolError),
            131 => Ok(ReturnCode::ImSpecificError),
            132 => Ok(ReturnCode::UnsupProtoVersion),
            133 => Ok(ReturnCode::ClientIdNotValid),
            134 => Ok(ReturnCode::BadUserNameOrPassword),
            135 => Ok(ReturnCode::NotAuthorizedV5),
            136 => Ok(ReturnCode::ServerUnavailableV5),
            137 => Ok(ReturnCode::ServerBusy),
            138 => Ok(ReturnCode::Banned),
            140 => Ok(ReturnCode::BadAuthMethod),
            142 => Ok(ReturnCode::SessionTakenOver),
            143 => Ok(ReturnCode::TopicFilterInvalid),
            144 => Ok(ReturnCode::TopicNameInvalid),
            146 => Ok(ReturnCode::PacketIdentifierNotFound),
            149 => Ok(ReturnCode::PacketTooLarge),
            151 => Ok(ReturnCode::QuotaExceeded),
            153 => Ok(ReturnCode::PayloadFormatInvalid),
            154 => Ok(ReturnCode::RetainNotSupported),
            155 => Ok(ReturnCode::QoSNotSupported),
            156 => Ok(ReturnCode::UseAnotherServer),
            157 => Ok(ReturnCode::ServerMoved),
            159 => Ok(ReturnCode::ConnectionRateExceeded),
            _ => Err(MqttProtocolError::InvalidReturnCode(value)),
        }
    }
}

impl std::fmt::Display for ReturnCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReturnCode::Success => write!(f, "0: Connection Accepted"),
            ReturnCode::UnsupportedProtocolVersion => {
                write!(f, "1: Connection Refused, Unacceptable Protocol Version")
            }
            ReturnCode::IdentifierRejected => {
                write!(f, "2: Connection Refused, Identifier Rejected")
            }
            ReturnCode::ServerUnavailable => {
                write!(f, "3: Connection Refused, Server Unavailable")
            }
            ReturnCode::InvalidUsernameOrPassword => {
                write!(f, "4: Connection Refused, Bad User Name or Password")
            }
            ReturnCode::NotAuthorized => write!(f, "5: Connection Refused, Not Authorized"),
            ReturnCode::NoMatchSubscription => write!(f, "16: No Matching Subscription"),
            ReturnCode::NoSubscriptionExisted => write!(f, "17: No Subscription Existed"),
            ReturnCode::UnspecifiedError => write!(f, "128: Unspecified Error"),
            ReturnCode::MalformedPacket => write!(f, "129: Malformed Packet"),
            ReturnCode::ProtocolError => write!(f, "130: Protocol Error"),
            ReturnCode::ImSpecificError => write!(f, "131: Implementation Specific Error"),
            ReturnCode::UnsupProtoVersion => {
                write!(f, "132: Unsupported Protocol Version")
            }
            ReturnCode::ClientIdNotValid => write!(f, "133: Client Identifier Not Valid"),
            ReturnCode::BadUserNameOrPassword => {
                write!(f, "134: Bad User Name or Password")
            }
            ReturnCode::NotAuthorizedV5 => write!(f, "135: Not Authorized"),
            ReturnCode::ServerUnavailableV5 => write!(f, "136: Server Unavailable"),
            ReturnCode::ServerBusy => write!(f, "137: Server Busy"),
            ReturnCode::Banned => write!(f, "138: Banned"),
            ReturnCode::BadAuthMethod => write!(f, "140: Bad Authentication Method"),
            ReturnCode::SessionTakenOver => write!(f, "142: Session Taken Over"),
            ReturnCode::TopicFilterInvalid => write!(f, "143: Topic Filter Invalid"),
            ReturnCode::TopicNameInvalid => write!(f, "144: Topic Name Invalid"),
            ReturnCode::PacketIdentifierNotFound => {
                write!(f, "146: Packet Identifier Not Found")
            }
            ReturnCode::PacketTooLarge => write!(f, "149: Packet Too Large"),
            ReturnCode::QuotaExceeded => write!(f, "151: Quota Exceeded"),
            ReturnCode::PayloadFormatInvalid => write!(f, "153: Payload Format Invalid"),
            ReturnCode::RetainNotSupported => write!(f, "154: Retain Not Supported"),
            ReturnCode::QoSNotSupported => write!(f, "155: QoS Not Supported"),
            ReturnCode::UseAnotherServer => write!(f, "156: Use Another Server"),
            ReturnCode::ServerMoved => write!(f, "157: Server Moved"),
            ReturnCode::ConnectionRateExceeded => {
                write!(f, "159: Connection Rate Exceeded")
            }
        }
    }
}
