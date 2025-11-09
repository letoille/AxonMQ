use std::collections::HashMap;

use crate::utils::time::now_milliseconds;

use super::error::SpbError;
use super::model::{
    metric::{DataMetric, Metric},
    value::Value,
};
use super::proto::{DataType, Payload};

#[allow(dead_code)]
pub enum MessageType {
    NodeBirth {
        seq: u8,
        timestamp: u64,
        bd_seq: u64,
        metrics: HashMap<String, Metric>,
    },
    NodeDeath {
        timestamp: u64,
        bd_seq: u64,
    },
    DeviceBirth {
        seq: u8,
        timestamp: u64,
        metrics: HashMap<String, Metric>,
    },
    DeviceDeath {
        seq: u8,
        timestamp: u64,
    },
    NodeData {
        seq: u8,
        timestamp: u64,
        metrics: Vec<DataMetric>,
    },
    DeviceData {
        seq: u8,
        timestamp: u64,
        metrics: Vec<DataMetric>,
    },
    NodeCommand {
        seq: u8,
        timestamp: u64,
    },
    DeviceCommand {
        seq: u8,
        timestamp: u64,
    },
}

pub struct Message {
    pub group_id: String,
    pub node_id: String,
    pub device_id: Option<String>,

    pub msg: MessageType,
}

impl Message {
    pub fn parse_nbirth(
        group_id: impl ToString,
        node_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidTimeStamp)?;
        let metrics = payload
            .metrics
            .iter()
            .map(|m| {
                let metric = Metric::try_from(m)?;
                Ok((metric.name.clone(), metric))
            })
            .collect::<Result<HashMap<String, Metric>, SpbError>>()?;

        let bd_seq;
        if let Some(bd_seq_metric) = metrics.get("bdSeq") {
            if bd_seq_metric.datatype != DataType::UInt64 as u32
                && bd_seq_metric.datatype != DataType::Int64 as u32
                && bd_seq_metric.datatype != DataType::UInt32 as u32
                && bd_seq_metric.datatype != DataType::Int32 as u32
                && bd_seq_metric.datatype != DataType::UInt16 as u32
                && bd_seq_metric.datatype != DataType::Int16 as u32
                && bd_seq_metric.datatype != DataType::UInt8 as u32
                && bd_seq_metric.datatype != DataType::Int8 as u32
            {
                return Err(SpbError::InvalidbdSeq);
            }
            match &bd_seq_metric.value {
                Some(value) => match value {
                    Value::Int64(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt64(v) => {
                        bd_seq = v.clone();
                    }
                    Value::Int32(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt32(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::Int16(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt16(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::Int8(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt8(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    _ => return Err(SpbError::InvalidbdSeq),
                },
                None => return Err(SpbError::BdSeqIsNone),
            };
        } else {
            return Err(SpbError::BdSeqNotFound);
        }

        if let Some(rebirth_metric) = metrics.get("Node Control/Rebirth") {
            if rebirth_metric.datatype != DataType::Boolean as u32 {
                return Err(SpbError::InvalidNodeRebirth);
            }
        } else {
            return Err(SpbError::InvalidNodeRebirth);
        }

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: None,
            msg: MessageType::NodeBirth {
                seq: seq as u8,
                timestamp: timestamp,
                bd_seq,
                metrics,
            },
        };

        Ok(msg)
    }

    pub fn parse_ndeath(
        group_id: impl ToString,
        node_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        if payload.seq.is_some() {
            return Err(SpbError::InvalidSeq);
        }

        let metrics = payload
            .metrics
            .iter()
            .map(|m| {
                let metric = Metric::try_from(m)?;
                Ok((metric.name.clone(), metric))
            })
            .collect::<Result<HashMap<String, Metric>, SpbError>>()?;

        let bd_seq;
        if let Some(bd_seq_metric) = metrics.get("bdSeq") {
            if bd_seq_metric.datatype != DataType::UInt64 as u32
                && bd_seq_metric.datatype != DataType::Int64 as u32
                && bd_seq_metric.datatype != DataType::UInt32 as u32
                && bd_seq_metric.datatype != DataType::Int32 as u32
                && bd_seq_metric.datatype != DataType::UInt16 as u32
                && bd_seq_metric.datatype != DataType::Int16 as u32
                && bd_seq_metric.datatype != DataType::UInt8 as u32
                && bd_seq_metric.datatype != DataType::Int8 as u32
            {
                return Err(SpbError::InvalidPayload);
            }
            match &bd_seq_metric.value {
                Some(value) => match value {
                    Value::Int64(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt64(v) => {
                        bd_seq = v.clone();
                    }
                    Value::Int32(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt32(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::Int16(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt16(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::Int8(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    Value::UInt8(v) => {
                        bd_seq = v.clone() as u64;
                    }
                    _ => return Err(SpbError::InvalidPayload),
                },
                None => return Err(SpbError::InvalidPayload),
            };
        } else {
            return Err(SpbError::InvalidPayload);
        }

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: None,
            msg: MessageType::NodeDeath {
                timestamp: now_milliseconds(),
                bd_seq,
            },
        };

        Ok(msg)
    }

    pub fn parse_ndata(
        group_id: impl ToString,
        node_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidPayload)?;
        let metrics = payload
            .metrics
            .iter()
            .map(|m| {
                let metric = DataMetric::try_from(m)?;
                if metric.name.is_none() && metric.alias.is_none() {
                    Err(SpbError::InvalidMetric)
                } else {
                    Ok(metric)
                }
            })
            .collect::<Result<Vec<DataMetric>, SpbError>>()?;

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: None,
            msg: MessageType::NodeData {
                seq: seq as u8,
                timestamp,
                metrics,
            },
        };

        Ok(msg)
    }

    pub fn parse_ncmd(
        group_id: impl ToString,
        node_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidPayload)?;

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: None,
            msg: MessageType::NodeCommand {
                seq: seq as u8,
                timestamp,
            },
        };

        Ok(msg)
    }

    pub fn parse_dbirth(
        group_id: impl ToString,
        node_id: impl ToString,
        device_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidPayload)?;
        let metrics = payload
            .metrics
            .iter()
            .map(|m| {
                let metric = Metric::try_from(m)?;
                Ok((metric.name.clone(), metric))
            })
            .collect::<Result<HashMap<String, Metric>, SpbError>>()?;

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: Some(device_id.to_string()),
            msg: MessageType::DeviceBirth {
                seq: seq as u8,
                timestamp,
                metrics,
            },
        };

        Ok(msg)
    }

    pub fn parse_ddeath(
        group_id: impl ToString,
        node_id: impl ToString,
        device_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidPayload)?;

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: Some(device_id.to_string()),
            msg: MessageType::DeviceDeath {
                seq: seq as u8,
                timestamp,
            },
        };

        Ok(msg)
    }

    pub fn parse_ddata(
        group_id: impl ToString,
        node_id: impl ToString,
        device_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidPayload)?;
        let metrics = payload
            .metrics
            .iter()
            .map(|m| {
                let metric = DataMetric::try_from(m)?;
                if metric.name.is_none() && metric.alias.is_none() {
                    Err(SpbError::InvalidMetric)
                } else {
                    Ok(metric)
                }
            })
            .collect::<Result<Vec<DataMetric>, SpbError>>()?;

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: Some(device_id.to_string()),
            msg: MessageType::DeviceData {
                seq: seq as u8,
                timestamp,
                metrics,
            },
        };

        Ok(msg)
    }

    pub fn parse_dcmd(
        group_id: impl ToString,
        node_id: impl ToString,
        device_id: impl ToString,
        payload: &Payload,
    ) -> Result<Self, SpbError> {
        let seq = payload.seq.ok_or(SpbError::InvalidSeq)?;
        if seq > 255 {
            return Err(SpbError::InvalidSeq);
        }
        let timestamp = payload.timestamp.ok_or(SpbError::InvalidPayload)?;

        let msg = Message {
            group_id: group_id.to_string(),
            node_id: node_id.to_string(),
            device_id: Some(device_id.to_string()),
            msg: MessageType::DeviceCommand {
                seq: seq as u8,
                timestamp,
            },
        };

        Ok(msg)
    }
}
