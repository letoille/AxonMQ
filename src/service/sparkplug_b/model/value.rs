use std::collections::HashMap;

use serde::Serialize;

use super::super::error::SpbError;
use super::super::proto::payload;
use super::metric::{DataMetric, Metric};
use super::template::{Template, TemplateDataInstance, TemplateInstance};

#[derive(Clone)]
pub enum Value {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    Boolean(bool),
    String(String),
    DateTime(u64), // Unix epoch time in milliseconds
    Text(String),  // UTF-8 encoded text

    UUID(String), // UTF-8 string
    //DataSet(DataSetMetric),
    Bytes(Vec<u8>), // Raw byte array
    File(Vec<u8>),  // Raw byte array representing a file

    Template(Template),
    TemplateDataInstance(TemplateDataInstance),
    TemplateInstance(TemplateInstance),

    Int8Array(Vec<i8>),
    Int16Array(Vec<i16>),
    Int32Array(Vec<i32>),
    Int64Array(Vec<i64>),
    UInt8Array(Vec<u8>),
    UInt16Array(Vec<u16>),
    UInt32Array(Vec<u32>),
    UInt64Array(Vec<u64>),
    FloatArray(Vec<f32>),
    DoubleArray(Vec<f64>),
    BooleanArray(Vec<bool>),
    StringArray(Vec<String>),
    DateTimeArray(Vec<u64>), // Array of Unix epoch times in milliseconds
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Int8(v) => serializer.serialize_i8(*v),
            Value::Int16(v) => serializer.serialize_i16(*v),
            Value::Int32(v) => serializer.serialize_i32(*v),
            Value::Int64(v) => serializer.serialize_i64(*v),
            Value::UInt8(v) => serializer.serialize_u8(*v),
            Value::UInt16(v) => serializer.serialize_u16(*v),
            Value::UInt32(v) => serializer.serialize_u32(*v),
            Value::UInt64(v) => serializer.serialize_u64(*v),
            Value::Float(v) => serializer.serialize_f32(*v),
            Value::Double(v) => serializer.serialize_f64(*v),
            Value::Boolean(v) => serializer.serialize_bool(*v),
            Value::String(v) => serializer.serialize_str(v),
            Value::DateTime(v) => serializer.serialize_u64(*v),
            Value::Text(v) => serializer.serialize_str(v),
            Value::UUID(v) => serializer.serialize_str(v),
            Value::Bytes(v) => serializer.serialize_bytes(v),
            Value::File(v) => serializer.serialize_bytes(v),
            Value::Template(_) => Err(serde::ser::Error::custom(
                "Template serialization not supported",
            )),
            Value::TemplateDataInstance(_) => Err(serde::ser::Error::custom(
                "TemplateDataInstance serialization not supported",
            )),
            Value::TemplateInstance(_) => Err(serde::ser::Error::custom(
                "TemplateInstance serialization not supported",
            )),
            Value::Int8Array(v) => v.serialize(serializer),
            Value::Int16Array(v) => v.serialize(serializer),
            Value::Int32Array(v) => v.serialize(serializer),
            Value::Int64Array(v) => v.serialize(serializer),
            Value::UInt8Array(v) => v.serialize(serializer),
            Value::UInt16Array(v) => v.serialize(serializer),
            Value::UInt32Array(v) => v.serialize(serializer),
            Value::UInt64Array(v) => v.serialize(serializer),
            Value::FloatArray(v) => v.serialize(serializer),
            Value::DoubleArray(v) => v.serialize(serializer),
            Value::BooleanArray(v) => v.serialize(serializer),
            Value::StringArray(v) => v.serialize(serializer),
            Value::DateTimeArray(v) => v.serialize(serializer),
        }
    }
}

impl TryFrom<(payload::metric::Value, Option<u32>)> for Value {
    type Error = SpbError;

    fn try_from((value, tp): (payload::metric::Value, Option<u32>)) -> Result<Self, Self::Error> {
        use payload::metric::Value as MV;

        match value {
            MV::IntValue(v) => match tp {
                Some(1) => Ok(Value::Int8(v as i8)),
                Some(2) => Ok(Value::Int16(v as i16)),
                Some(3) => Ok(Value::Int32(v as i32)),
                Some(5) => Ok(Value::UInt8(v as u8)),
                Some(6) => Ok(Value::UInt16(v as u16)),
                Some(7) => Ok(Value::UInt32(v)),
                None => Ok(Value::UInt32(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::LongValue(v) => match tp {
                Some(4) => Ok(Value::Int64(v as i64)),
                Some(8) => Ok(Value::UInt64(v)),
                Some(11) => Ok(Value::DateTime(v)),
                None => Ok(Value::UInt64(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::FloatValue(v) => match tp {
                Some(9) => Ok(Value::Float(v)),
                None => Ok(Value::Float(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::DoubleValue(v) => match tp {
                Some(10) => Ok(Value::Double(v)),
                None => Ok(Value::Double(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::BooleanValue(v) => match tp {
                Some(11) => Ok(Value::Boolean(v)),
                None => Ok(Value::Boolean(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::StringValue(v) => match tp {
                Some(12) => Ok(Value::String(v)),
                Some(14) => Ok(Value::Text(v)),
                Some(15) => Ok(Value::UUID(v)),
                None => Ok(Value::String(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::BytesValue(v) => match tp {
                Some(17) => Ok(Value::Bytes(v)),
                Some(18) => Ok(Value::File(v)),
                Some(22) => Ok(Value::Int8Array(v.into_iter().map(|v| v as i8).collect())),
                Some(23) => {
                    let mut vi16 = Vec::new();
                    for i in v.chunks(2) {
                        let arr = [i[0], i[1]];
                        vi16.push(i16::from_le_bytes(arr));
                    }
                    Ok(Value::Int16Array(vi16))
                }
                Some(24) => {
                    let mut vi32 = Vec::new();
                    for i in v.chunks(4) {
                        let arr = [i[0], i[1], i[2], i[3]];
                        vi32.push(i32::from_le_bytes(arr));
                    }
                    Ok(Value::Int32Array(vi32))
                }
                Some(25) => {
                    let mut vi64 = Vec::new();
                    for i in v.chunks(8) {
                        let arr = [i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7]];
                        vi64.push(i64::from_le_bytes(arr));
                    }
                    Ok(Value::Int64Array(vi64))
                }
                Some(26) => Ok(Value::UInt8Array(v)),
                Some(27) => {
                    let mut vu16 = Vec::new();
                    for i in v.chunks(2) {
                        let arr = [i[0], i[1]];
                        vu16.push(u16::from_le_bytes(arr));
                    }
                    Ok(Value::UInt16Array(vu16))
                }
                Some(28) => {
                    let mut vu32 = Vec::new();
                    for i in v.chunks(4) {
                        let arr = [i[0], i[1], i[2], i[3]];
                        vu32.push(u32::from_le_bytes(arr));
                    }
                    Ok(Value::UInt32Array(vu32))
                }
                Some(29) => {
                    let mut vu64 = Vec::new();
                    for i in v.chunks(8) {
                        let arr = [i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7]];
                        vu64.push(u64::from_le_bytes(arr));
                    }
                    Ok(Value::UInt64Array(vu64))
                }
                Some(30) => {
                    let mut vf32 = Vec::new();
                    for i in v.chunks(4) {
                        let arr = [i[0], i[1], i[2], i[3]];
                        vf32.push(f32::from_le_bytes(arr));
                    }
                    Ok(Value::FloatArray(vf32))
                }
                Some(31) => {
                    let mut vf64 = Vec::new();
                    for i in v.chunks(8) {
                        let arr = [i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7]];
                        vf64.push(f64::from_le_bytes(arr));
                    }
                    Ok(Value::DoubleArray(vf64))
                }
                Some(32) => {
                    let mut vb = Vec::new();
                    for byte in v {
                        vb.push(byte != 0);
                    }
                    Ok(Value::BooleanArray(vb))
                }
                Some(33) => {
                    let s = String::from_utf8(v).map_err(|_| SpbError::InvalidDataType)?;
                    let vec_str: Vec<String> = s.split('\n').map(|s| s.to_string()).collect();
                    Ok(Value::StringArray(vec_str))
                }
                Some(34) => {
                    let mut vdt = Vec::new();
                    for i in v.chunks(8) {
                        let arr = [i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7]];
                        vdt.push(u64::from_le_bytes(arr));
                    }
                    Ok(Value::DateTimeArray(vdt))
                }
                None => Ok(Value::Bytes(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            MV::TemplateValue(v) => match tp {
                Some(19) => {
                    if v.is_definition.is_none() {
                        return Err(SpbError::InvalidTemplate);
                    }

                    if v.is_definition.unwrap() {
                        if v.template_ref.is_some() {
                            return Err(SpbError::InvalidTemplate);
                        }

                        let mut metrics = HashMap::new();
                        let mut alias: HashMap<u64, String> = HashMap::new();

                        for m in v.metrics {
                            let metric = Metric::try_from(&m)?;

                            if let Some(a) = metric.alias {
                                alias.insert(a, metric.name.clone());
                            }
                            metrics.insert(metric.name.clone(), metric);
                        }

                        let define = Template {
                            name: "".to_string(),
                            version: v.version,
                            metrics,
                            alias,
                            in_properties: vec![],
                            properties: vec![],
                        };
                        Ok(Value::Template(define))
                    } else {
                        if v.template_ref.is_none() {
                            return Err(SpbError::InvalidTemplateInstance);
                        }

                        let metrics = v
                            .metrics
                            .into_iter()
                            .map(|m| DataMetric::try_from(&m))
                            .collect::<Result<Vec<DataMetric>, SpbError>>()?;

                        let instance = TemplateDataInstance {
                            template_ref: v.template_ref.clone().unwrap(),
                            version: v.version,
                            metrics,
                        };

                        Ok(Value::TemplateDataInstance(instance))
                    }
                }
                _ => Err(SpbError::InvalidDataType),
            },
            _ => {
                unimplemented!("conversion for complex types not implemented yet");
            }
        }
    }
}
