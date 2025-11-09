use bytes::Bytes;

use crate::utils::time::now_milliseconds;

use super::proto;

pub struct Cmd {
    pub seq: u8,
}

impl Cmd {
    pub fn new() -> Self {
        Self { seq: 0 }
    }

    fn next_seq(&mut self) -> u64 {
        let current = self.seq;
        self.seq = self.seq.wrapping_add(1);
        current as u64
    }

    pub fn node_rebirth(
        &mut self,
        group_id: String,
        node_id: String,
        alias: Option<u64>,
    ) -> (String, Bytes) {
        use prost::Message as _;

        let topic = format!("spBv1.0/{}/NCMD/{}", group_id, node_id);
        let metric = proto::payload::Metric {
            name: Some("Node Control/Rebirth".to_string()),
            alias,
            datatype: Some(proto::DataType::Boolean as u32),
            value: Some(proto::payload::metric::Value::BooleanValue(true)),
            timestamp: None,
            properties: None,
            is_historical: None,
            is_null: None,
            is_transient: None,
            metadata: None,
        };

        let payload = proto::Payload {
            uuid: None,
            body: None,
            timestamp: Some(now_milliseconds()),
            seq: Some(self.next_seq()),
            metrics: vec![metric],
        }
        .encode_to_vec();

        (topic, Bytes::from(payload))
    }
}
