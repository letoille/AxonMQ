use bytes::Bytes;

use super::super::QoS;
use super::property::Property;

#[derive(Clone)]
pub(crate) struct Will {
    pub(crate) topic: String,
    pub(crate) payload: Bytes,
    pub(crate) qos: QoS,
    pub(crate) retain: bool,

    pub(crate) will_delay_interval: u32,
    pub(crate) expiry_interval: Option<u64>,

    pub(crate) properties: Vec<Property>,
}
