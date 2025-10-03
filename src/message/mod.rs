use std::collections::HashMap;

use bytes::Bytes;

use crate::mqtt::{QoS, protocol::property::Property};

#[derive(Clone, PartialEq)]
pub enum MetadataValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Bytes),
}

#[derive(Clone)]
pub struct Message {
    pub client_id: String,
    pub topic: String,
    pub qos: QoS,
    pub retain: bool,
    pub expiry_at: Option<u64>,

    pub payload: Bytes,
    pub properties: Vec<Property>,

    pub metadata: HashMap<String, MetadataValue>,
}

impl Message {
    pub fn new(
        client_id: String,
        topic: String,
        qos: QoS,
        retain: bool,
        expiry_at: Option<u64>,
        payload: Bytes,
        properties: Vec<Property>,
    ) -> Self {
        Message {
            client_id,
            topic,
            qos,
            retain,
            expiry_at,
            payload,
            properties,
            metadata: HashMap::new(),
        }
    }
}
