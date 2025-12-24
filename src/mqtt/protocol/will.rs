use bytes::Bytes;

use super::super::QoS;
use super::property::PropertyUser;

#[derive(Clone)]
pub struct WillOptions {
    pub options: super::publish::PublishOptions,
    pub will_delay_interval: Option<u32>,
}

impl From<super::publish::PublishOptions> for WillOptions {
    fn from(options: super::publish::PublishOptions) -> Self {
        Self {
            options,
            will_delay_interval: None,
        }
    }
}

impl std::default::Default for WillOptions {
    fn default() -> Self {
        Self {
            options: super::publish::PublishOptions::default(),
            will_delay_interval: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Will {
    pub(crate) topic: String,
    pub(crate) payload: Bytes,
    pub(crate) qos: QoS,
    pub(crate) retain: bool,

    pub(crate) user_properties: Vec<PropertyUser>,
    pub(crate) options: WillOptions,
}
