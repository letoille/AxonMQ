use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use super::super::error::SpbError;
use super::super::in_helper::FlattenValue;
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
                    let len = u32::from_le_bytes([v[0], v[1], v[2], v[3]]);

                    for i in 0..len {
                        let byte_index = (i / 8) as usize + 4;
                        let bit_index = 7 - (i % 8);
                        let bit = (v[byte_index] >> bit_index) & 1;
                        vb.push(bit == 1);
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

impl From<Value> for payload::metric::Value {
    fn from(value: Value) -> Self {
        use payload::metric::Value as MV;

        match value {
            Value::Int8(v) => MV::IntValue(v as u32),
            Value::Int16(v) => MV::IntValue(v as u32),
            Value::Int32(v) => MV::IntValue(v as u32),
            Value::Int64(v) => MV::LongValue(v as u64),
            Value::UInt8(v) => MV::IntValue(v as u32),
            Value::UInt16(v) => MV::IntValue(v as u32),
            Value::UInt32(v) => MV::IntValue(v as u32),
            Value::UInt64(v) => MV::LongValue(v),
            Value::Float(v) => MV::FloatValue(v),
            Value::Double(v) => MV::DoubleValue(v),
            Value::Boolean(v) => MV::BooleanValue(v),
            Value::String(v) => MV::StringValue(v),
            Value::DateTime(v) => MV::LongValue(v),
            Value::Text(v) => MV::StringValue(v),
            Value::UUID(v) => MV::StringValue(v),
            Value::Bytes(v) => MV::BytesValue(v),
            Value::File(v) => MV::BytesValue(v),
            Value::BooleanArray(v) => {
                let mut bytes = Vec::new();
                let len = v.len() as u32;
                bytes.extend_from_slice(&len.to_le_bytes());

                let mut byte = 0u8;
                let mut offset = 0;
                for (of, b) in v.iter().enumerate() {
                    if *b {
                        byte |= 1 << (7 - of % 8);
                    }

                    offset += 1;
                    if offset == 8 {
                        bytes.push(byte);
                        byte = 0;
                        offset = 0;
                    }
                }

                MV::BytesValue(bytes)
            }
            Value::Int8Array(v) => MV::BytesValue(v.into_iter().map(|i| i as u8).collect()),
            Value::UInt8Array(v) => MV::BytesValue(v),
            Value::Int16Array(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::UInt16Array(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::Int32Array(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::UInt32Array(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::Int64Array(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::UInt64Array(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::FloatArray(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::DoubleArray(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::StringArray(v) => {
                let s = v.join("\n");
                MV::BytesValue(s.into_bytes())
            }
            Value::DateTimeArray(v) => {
                let mut bytes = Vec::new();
                for i in v.iter() {
                    bytes.extend_from_slice(&i.to_le_bytes());
                }
                MV::BytesValue(bytes)
            }
            Value::TemplateDataInstance(instance) => {
                let mut metrics = Vec::new();
                for m in instance.metrics.into_iter() {
                    let pm: payload::Metric = m.into();
                    metrics.push(pm);
                }

                let tv = payload::Template {
                    parameters: vec![],
                    is_definition: Some(false),
                    template_ref: Some(instance.template_ref),
                    version: instance.version,
                    metrics,
                };

                MV::TemplateValue(tv)
            }
            _ => {
                unreachable!("conversion for complex types not implemented yet");
            }
        }
    }
}

impl TryFrom<(FlattenValue, u32)> for Value {
    type Error = SpbError;

    fn try_from((fv, tp): (FlattenValue, u32)) -> Result<Self, Self::Error> {
        match fv {
            FlattenValue::Bool(v) => {
                if tp == 11 {
                    Ok(Value::Boolean(v))
                } else {
                    Err(SpbError::InvalidValue)
                }
            }
            FlattenValue::Int(v) => match tp {
                1 => {
                    if v < i8::MIN as i64 || v > i8::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int8(v as i8))
                    }
                }
                2 => {
                    if v < i16::MIN as i64 || v > i16::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int16(v as i16))
                    }
                }
                3 => {
                    if v < i32::MIN as i64 || v > i32::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int32(v as i32))
                    }
                }
                4 => Ok(Value::Int64(v)),
                5 => {
                    if v < 0 || v > u8::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt8(v as u8))
                    }
                }
                6 => {
                    if v < 0 || v > u16::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt16(v as u16))
                    }
                }
                7 => {
                    if v < 0 || v > u32::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt32(v as u32))
                    }
                }
                8 => {
                    if v < 0 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt64(v as u64))
                    }
                }
                9 => {
                    if v < f32::MIN as i64 || v > f32::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Float(v as f32))
                    }
                }
                10 => {
                    if v < f64::MIN as i64 || v > f64::MAX as i64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Double(v as f64))
                    }
                }
                13 => {
                    if v < 0 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::DateTime(v as u64))
                    }
                }
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::UInt(v) => match tp {
                1 => {
                    if v > i8::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int8(v as i8))
                    }
                }
                2 => {
                    if v > i16::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int16(v as i16))
                    }
                }
                3 => {
                    if v > i32::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int32(v as i32))
                    }
                }
                4 => {
                    if v > i64::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Int64(v as i64))
                    }
                }
                5 => {
                    if v > u8::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt8(v as u8))
                    }
                }
                6 => {
                    if v > u16::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt16(v as u16))
                    }
                }
                7 => {
                    if v > u32::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::UInt32(v as u32))
                    }
                }
                8 => Ok(Value::UInt64(v)),
                9 => {
                    if v > f32::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Float(v as f32))
                    }
                }
                10 => {
                    if v > f64::MAX as u64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Double(v as f64))
                    }
                }
                13 => Ok(Value::DateTime(v)),
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::String(v) => match tp {
                12 => Ok(Value::String(v)),
                14 => Ok(Value::Text(v)),
                15 => {
                    if Uuid::parse_str(&v).is_err() {
                        return Err(SpbError::InvalidValue);
                    }
                    Ok(Value::UUID(v))
                }
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::Float(v) => match tp {
                9 => {
                    if v < f32::MIN as f64 || v > f32::MAX as f64 {
                        Err(SpbError::InvalidValue)
                    } else {
                        Ok(Value::Float(v as f32))
                    }
                }
                10 => Ok(Value::Double(v as f64)),
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::ArrayBool(v) => {
                if tp == 32 {
                    Ok(Value::BooleanArray(v))
                } else {
                    Err(SpbError::InvalidValue)
                }
            }
            FlattenValue::ArrayInt(v) => match tp {
                17 => {
                    let mut bytes = Vec::new();
                    for i in v.iter() {
                        if *i < 0 || *i > u8::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        bytes.push(*i as u8);
                    }
                    Ok(Value::Bytes(bytes))
                }
                22 => {
                    let mut vi8 = Vec::new();
                    for i in v.iter() {
                        if *i < i8::MIN as i64 || *i > i8::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi8.push(*i as i8);
                    }
                    Ok(Value::Int8Array(vi8))
                }
                23 => {
                    let mut vi16 = Vec::new();
                    for i in v.iter() {
                        if *i < i16::MIN as i64 || *i > i16::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi16.push(*i as i16);
                    }
                    Ok(Value::Int16Array(vi16))
                }
                24 => {
                    let mut vi32 = Vec::new();
                    for i in v.iter() {
                        if *i < i32::MIN as i64 || *i > i32::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi32.push(*i as i32);
                    }
                    Ok(Value::Int32Array(vi32))
                }
                25 => Ok(Value::Int64Array(v)),
                26 => {
                    let mut vu8 = Vec::new();
                    for i in v.iter() {
                        if *i < 0 || *i > u8::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu8.push(*i as u8);
                    }
                    Ok(Value::UInt8Array(vu8))
                }
                27 => {
                    let mut vu16 = Vec::new();
                    for i in v.iter() {
                        if *i < 0 || *i > u16::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu16.push(*i as u16);
                    }
                    Ok(Value::UInt16Array(vu16))
                }
                28 => {
                    let mut vu32 = Vec::new();
                    for i in v.iter() {
                        if *i < 0 || *i > u32::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu32.push(*i as u32);
                    }
                    Ok(Value::UInt32Array(vu32))
                }
                29 => {
                    let mut vu64 = Vec::new();
                    for i in v.iter() {
                        if *i < 0 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu64.push(*i as u64);
                    }
                    Ok(Value::UInt64Array(vu64))
                }
                30 => {
                    let mut vf32 = Vec::new();
                    for i in v.iter() {
                        if *i < f32::MIN as i64 || *i > f32::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vf32.push(*i as f32);
                    }
                    Ok(Value::FloatArray(vf32))
                }
                31 => {
                    let mut vf64 = Vec::new();
                    for i in v.iter() {
                        if *i < f64::MIN as i64 || *i > f64::MAX as i64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vf64.push(*i as f64);
                    }
                    Ok(Value::DoubleArray(vf64))
                }
                34 => {
                    let mut vdt = Vec::new();
                    for i in v.iter() {
                        if *i < 0 {
                            return Err(SpbError::InvalidValue);
                        }
                        vdt.push(*i as u64);
                    }
                    Ok(Value::DateTimeArray(vdt))
                }
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::ArrayUInt(v) => match tp {
                17 => {
                    let mut bytes = Vec::new();
                    for i in v.iter() {
                        if *i > u8::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        bytes.push(*i as u8);
                    }
                    Ok(Value::Bytes(bytes))
                }
                22 => {
                    let mut vi8 = Vec::new();
                    for i in v.iter() {
                        if *i > i8::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi8.push(*i as i8);
                    }
                    Ok(Value::Int8Array(vi8))
                }
                23 => {
                    let mut vi16 = Vec::new();
                    for i in v.iter() {
                        if *i > i16::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi16.push(*i as i16);
                    }
                    Ok(Value::Int16Array(vi16))
                }
                24 => {
                    let mut vi32 = Vec::new();
                    for i in v.iter() {
                        if *i > i32::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi32.push(*i as i32);
                    }
                    Ok(Value::Int32Array(vi32))
                }
                25 => {
                    let mut vi64 = Vec::new();
                    for i in v.iter() {
                        if *i > i64::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vi64.push(*i as i64);
                    }
                    Ok(Value::Int64Array(vi64))
                }
                26 => {
                    let mut vu8 = Vec::new();
                    for i in v.iter() {
                        if *i > u8::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu8.push(*i as u8);
                    }
                    Ok(Value::UInt8Array(vu8))
                }
                27 => {
                    let mut vu16 = Vec::new();
                    for i in v.iter() {
                        if *i > u16::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu16.push(*i as u16);
                    }
                    Ok(Value::UInt16Array(vu16))
                }
                28 => {
                    let mut vu32 = Vec::new();
                    for i in v.iter() {
                        if *i > u32::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vu32.push(*i as u32);
                    }
                    Ok(Value::UInt32Array(vu32))
                }
                29 => Ok(Value::UInt64Array(v)),
                30 => {
                    let mut vf32 = Vec::new();
                    for i in v.iter() {
                        if *i > f32::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vf32.push(*i as f32);
                    }
                    Ok(Value::FloatArray(vf32))
                }
                31 => {
                    let mut vf64 = Vec::new();
                    for i in v.iter() {
                        if *i > f64::MAX as u64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vf64.push(*i as f64);
                    }
                    Ok(Value::DoubleArray(vf64))
                }
                34 => {
                    let mut vdt = Vec::new();
                    for i in v.iter() {
                        if *i > u64::MAX {
                            return Err(SpbError::InvalidValue);
                        }
                        vdt.push(*i as u64);
                    }
                    Ok(Value::DateTimeArray(vdt))
                }
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::ArrayFloat(v) => match tp {
                9 => {
                    let mut vf32 = Vec::new();
                    for i in v.iter() {
                        if *i < f32::MIN as f64 || *i > f32::MAX as f64 {
                            return Err(SpbError::InvalidValue);
                        }
                        vf32.push(*i as f32);
                    }
                    Ok(Value::FloatArray(vf32))
                }
                10 => Ok(Value::DoubleArray(v)),
                _ => Err(SpbError::InvalidValue),
            },
            FlattenValue::ArrayString(v) => {
                if tp == 33 {
                    Ok(Value::StringArray(v))
                } else {
                    Err(SpbError::InvalidValue)
                }
            }
        }
    }
}
