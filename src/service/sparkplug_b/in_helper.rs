use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::Visitor};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

use crate::error::AxonError;

use crate::service::sparkplug_b::model::{metric::Metric, template::Template};

#[derive(Clone, Serialize)]
pub struct GetNodeResponse {
    pub node_id: String,
    pub online: bool,
    pub timestamp: u64,

    pub setting: Vec<Metric>,
    pub metrics: Vec<Metric>,

    pub templates: HashMap<String, Template>,
}

#[derive(Clone, Serialize)]
pub struct GetDeviceResponse {
    pub device: String,
    pub online: bool,
    pub timestamp: u64,

    pub setting: Vec<Metric>,
    pub metrics: Vec<Metric>,
}

#[derive(Clone)]
pub enum FlattenValue {
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    ArrayBool(Vec<bool>),
    ArrayInt(Vec<i64>),
    ArrayUInt(Vec<u64>),
    ArrayFloat(Vec<f64>),
    ArrayString(Vec<String>),
}

struct FlattenValueVisitor;

impl<'de> Visitor<'de> for FlattenValueVisitor {
    type Value = FlattenValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a flatten value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::Int(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::UInt(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::Float(v))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::String(v.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::String(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(FlattenValue::String(v))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec_bool: Vec<(u16, bool)> = Vec::new();
        let mut vec_i64: Vec<(u16, i64)> = Vec::new();
        let mut vec_u64: Vec<(u16, u64)> = Vec::new();
        let mut vec_f64: Vec<(u16, f64)> = Vec::new();
        let mut vec_str: Vec<(u16, String)> = Vec::new();
        let mut index: u16 = 0;

        while let Some(value) = seq.next_element::<FlattenValue>()? {
            match value {
                FlattenValue::Bool(v) => {
                    vec_bool.push((index, v));
                }
                FlattenValue::UInt(v) => {
                    vec_u64.push((index, v));
                }
                FlattenValue::Int(v) => {
                    vec_i64.push((index, v));
                }
                FlattenValue::Float(v) => {
                    vec_f64.push((index, v));
                }
                FlattenValue::String(v) => {
                    vec_str.push((index, v));
                }
                _ => return Err(serde::de::Error::custom("array item invalid type")),
            }
            index += 1;
        }

        if vec_bool.len() > 0 && vec_i64.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }

        if vec_bool.len() > 0 && vec_u64.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }

        if vec_bool.len() > 0 && vec_f64.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }
        if vec_bool.len() > 0 && vec_str.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }
        if vec_str.len() > 0 && vec_i64.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }

        if vec_str.len() > 0 && vec_u64.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }

        if vec_str.len() > 0 && vec_f64.len() > 0 {
            return Err(serde::de::Error::custom("invalid array"));
        }

        if vec_bool.len() > 0 {
            let vec_bool = vec_bool.into_iter().map(|(_, v)| v).collect();
            return Ok(FlattenValue::ArrayBool(vec_bool));
        }

        if vec_str.len() > 0 {
            let vec_str = vec_str.into_iter().map(|(_, v)| v).collect();
            return Ok(FlattenValue::ArrayString(vec_str));
        }

        if vec_f64.len() > 0 {
            let mut vec: Vec<f64> = vec![0.0; vec_f64.len() + vec_i64.len() + vec_u64.len()];

            for (i, v) in vec_f64 {
                vec[i as usize] = v;
            }
            for (i, v) in vec_i64 {
                vec[i as usize] = v as f64;
            }
            for (i, v) in vec_u64 {
                vec[i as usize] = v as f64;
            }

            return Ok(FlattenValue::ArrayFloat(vec));
        }

        if vec_i64.len() > 0 {
            let mut vec: Vec<i64> = vec![0; vec_i64.len() + vec_u64.len()];

            for (i, v) in vec_i64 {
                vec[i as usize] = v;
            }
            for (i, v) in vec_u64 {
                vec[i as usize] = v as i64;
            }

            return Ok(FlattenValue::ArrayInt(vec));
        }

        if vec_u64.len() > 0 {
            let vec = vec_u64.into_iter().map(|(_, v)| v).collect();
            return Ok(FlattenValue::ArrayUInt(vec));
        }

        return Err(serde::de::Error::custom("array is empty"));
    }
}

impl<'de> Deserialize<'de> for FlattenValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FlattenValueVisitor)
    }
}

#[derive(Clone, Deserialize)]
pub struct KV {
    pub template: Option<String>,
    pub name: String,
    pub value: FlattenValue,
}

#[derive(Clone, Deserialize)]
pub struct SetNodeRequest {
    pub group_id: String,
    pub node_id: String,
    pub kvs: Vec<KV>,
}

#[derive(Clone, Deserialize)]
pub struct SetDeviceRequest {
    pub group_id: String,
    pub node_id: String,
    pub device: String,
    pub kvs: Vec<KV>,
}

#[derive(Clone, Serialize)]
pub struct KE {
    pub name: String,
    pub error: String,
}

#[derive(Clone, Serialize)]
pub struct SetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    pub details: Vec<KE>,
}

pub enum InMessage {
    GetGroups {
        group: Option<String>,
        resp: oneshot::Sender<Result<Vec<String>, AxonError>>,
    },
    GetNodes {
        group_id: String,
        node_id: Option<String>,
        resp: oneshot::Sender<Result<Vec<GetNodeResponse>, AxonError>>,
    },
    GetDevice {
        group_id: String,
        node_id: String,
        device: Option<String>,
        resp: oneshot::Sender<Result<Vec<GetDeviceResponse>, AxonError>>,
    },
    SetNodeRequest {
        req: SetNodeRequest,
        resp: oneshot::Sender<Result<Vec<(String, String)>, AxonError>>,
    },
    SetDeviceRequest {
        req: SetDeviceRequest,
        resp: oneshot::Sender<Result<Vec<(String, String)>, AxonError>>,
    },
}

#[derive(Clone)]
pub struct InHelper {
    tx: mpsc::Sender<InMessage>,
}

impl InHelper {
    pub fn new(tx: mpsc::Sender<InMessage>) -> Self {
        InHelper { tx }
    }

    pub async fn get_groups(&self, group: Option<String>) -> Result<Vec<String>, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::GetGroups {
            group,
            resp: resp_tx,
        };

        timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??
    }

    pub async fn get_nodes(
        &self,
        group_id: String,
        node_id: Option<String>,
    ) -> Result<Vec<GetNodeResponse>, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::GetNodes {
            group_id,
            node_id,
            resp: resp_tx,
        };

        timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??
    }

    pub async fn get_devices(
        &self,
        group_id: String,
        node_id: String,
        device: Option<String>,
    ) -> Result<Vec<GetDeviceResponse>, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::GetDevice {
            group_id,
            node_id,
            device,
            resp: resp_tx,
        };

        timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??
    }

    pub async fn set_node(
        &self,
        group_id: String,
        node_id: String,
        kvs: Vec<KV>,
    ) -> Result<SetResponse, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::SetNodeRequest {
            req: SetNodeRequest {
                group_id,
                node_id,
                kvs,
            },
            resp: resp_tx,
        };

        let result = timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??;

        if let Err(e) = &result {
            return Ok(SetResponse {
                result: Some(e.to_string()),
                details: vec![],
            });
        }

        Ok(SetResponse {
            result: None,
            details: result
                .unwrap()
                .into_iter()
                .map(|(n, e)| KE { name: n, error: e })
                .collect(),
        })
    }

    pub async fn set_device(
        &self,
        group_id: String,
        node_id: String,
        device: String,
        kvs: Vec<KV>,
    ) -> Result<SetResponse, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::SetDeviceRequest {
            req: SetDeviceRequest {
                group_id,
                node_id,
                device,
                kvs,
            },
            resp: resp_tx,
        };

        let result = timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??;

        if let Err(e) = &result {
            return Ok(SetResponse {
                result: Some(e.to_string()),
                details: vec![],
            });
        }

        Ok(SetResponse {
            result: None,
            details: result
                .unwrap()
                .into_iter()
                .map(|(n, e)| KE { name: n, error: e })
                .collect(),
        })
    }
}
