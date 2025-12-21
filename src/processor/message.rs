use std::collections::HashMap;

use bytes::Bytes;
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::mqtt::{QoS, protocol::property::PropertyUser};
use crate::utils;

#[derive(Clone, PartialEq, Serialize)]
pub enum MetadataValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Json(JsonValue),
}

pub enum MetadataKey {
    PayloadFormat,
    ParsedPayloadJson,
}

pub enum MetadataPayloadFormat {
    Json,
    String,
}

impl MetadataKey {
    pub fn as_str(&self) -> &str {
        match self {
            MetadataKey::PayloadFormat => "__payload_format",
            MetadataKey::ParsedPayloadJson => "__parsed_payload_json",
        }
    }
}

impl MetadataPayloadFormat {
    pub fn as_str(&self) -> &str {
        match self {
            MetadataPayloadFormat::Json => "json",
            MetadataPayloadFormat::String => "string",
        }
    }
}

impl std::fmt::Display for MetadataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataValue::String(s) => write!(f, "{}", s),
            MetadataValue::Int(i) => write!(f, "{}", i),
            MetadataValue::Float(fl) => write!(f, "{}", fl),
            MetadataValue::Bool(b) => write!(f, "{}", b),
            MetadataValue::Json(json) => write!(f, "{}", json),
        }
    }
}

#[derive(Clone)]
pub struct Message {
    pub client_id: String,
    pub topic: String,
    pub qos: QoS,
    pub retain: bool,
    pub expiry_at: Option<u64>,

    pub payload: Bytes,
    pub user_properties: Vec<PropertyUser>,

    pub subscription_identifier: Option<u32>,

    pub metadata: HashMap<String, MetadataValue>,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "client_id: {}, topic: {}, qos: {}, retain: {}",
            utils::TruncateDisplay::new(&self.client_id, 128),
            utils::TruncateDisplay::new(&self.topic, 128),
            self.qos,
            self.retain,
        )?;

        if !self.metadata.is_empty() {
            write!(f, ", metadata: {{")?;
        }
        for (meta_key, meta_value) in &self.metadata {
            write!(f, "{}: {}, ", meta_key, meta_value)?;
        }
        if !self.metadata.is_empty() {
            write!(f, "}}")?;
        }

        write!(
            f,
            ", payload: {}",
            utils::BytesTruncated::new(&self.payload, 128)
        )
    }
}

impl Message {
    pub fn new(
        client_id: String,
        topic: String,
        qos: QoS,
        retain: bool,
        expiry_at: Option<u64>,
        payload: Bytes,
        user_properties: Vec<PropertyUser>,
    ) -> Self {
        Message {
            client_id,
            topic,
            qos,
            retain,
            expiry_at,
            payload,
            user_properties,
            metadata: HashMap::new(),
            subscription_identifier: None,
        }
    }

    pub fn with_subscription_identifier(mut self, subscription_identifier: Option<u32>) -> Self {
        self.subscription_identifier = subscription_identifier;
        self
    }
}
