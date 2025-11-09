use serde::Serialize;

use crate::utils::time::now_milliseconds;

use super::super::error::SpbError;
use super::super::proto::payload;
use super::value::Value;

#[derive(Clone, Serialize)]
pub enum MetricType {
    Data,
    Setting,
}

#[derive(Clone, Serialize)]
pub enum MetricProperty {
    Type(MetricType),
}

impl TryFrom<&Property> for MetricProperty {
    type Error = SpbError;

    fn try_from(prop: &Property) -> Result<Self, Self::Error> {
        if prop.keys.len() != 1 || prop.values.len() != 1 {
            return Err(SpbError::InvalidMetricProperty);
        }

        let key = &prop.keys[0];
        let value = &prop.values[0];

        match key.as_str() {
            "type" => {
                if let Some(ref tp) = value.2 {
                    match tp {
                        Value::String(s) if s == "data" => {
                            Ok(MetricProperty::Type(MetricType::Data))
                        }
                        Value::String(s) if s == "setting" => {
                            Ok(MetricProperty::Type(MetricType::Setting))
                        }
                        _ => Ok(MetricProperty::Type(MetricType::Data)),
                    }
                } else {
                    Ok(MetricProperty::Type(MetricType::Data))
                }
            }
            _ => Err(SpbError::InvalidMetricProperty),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct Metric {
    pub name: String,
    #[serde(skip)]
    pub alias: Option<u64>,
    pub timestamp: u64,
    pub datatype: u32,
    pub is_null: bool,
    pub stale: bool,
    pub value: Option<Value>,
    #[serde(skip)]
    pub in_property: Vec<MetricProperty>,
    pub properties: Vec<Property>,
}

impl Metric {
    pub fn is_setting(&self) -> bool {
        for prop in self.in_property.iter() {
            if let MetricProperty::Type(MetricType::Setting) = prop {
                return true;
            }
        }
        false
    }

    pub fn update(&mut self, other: DataMetric) -> Result<(), SpbError> {
        self.timestamp = other.timestamp;
        self.is_null = other.is_null.unwrap_or(false);

        if other.datatype.is_some() && other.datatype.unwrap() != self.datatype {
            return Err(SpbError::MetricNotMatch);
        }

        if other.value.is_some() && !self.is_null {
            if let Some(Value::TemplateInstance(ref mut instance)) = self.value {
                if let Some(Value::TemplateDataInstance(data_instance)) = other.value {
                    instance.update(&data_instance)?;
                } else {
                    return Err(SpbError::MetricNotMatch);
                }
            } else {
                self.value = other.value;
            }
        }

        if other.properties.len() > 0 {
            self.properties = other.properties;
        }

        self.stale = false;

        Ok(())
    }
}

impl TryFrom<&payload::Metric> for Metric {
    type Error = SpbError;

    fn try_from(pm: &payload::Metric) -> Result<Self, Self::Error> {
        let properties = pm
            .properties
            .iter()
            .map(|p| Property::try_from(p.clone()))
            .collect::<Result<Vec<Property>, SpbError>>()?;
        let value = if let Some(ref v) = pm.value {
            Some(Value::try_from((v.clone(), pm.datatype))?)
        } else {
            None
        };

        let mut new_properties = vec![];
        let in_property = properties
            .into_iter()
            .filter_map(|p| {
                let r = MetricProperty::try_from(&p);
                if r.is_ok() {
                    r.ok()
                } else {
                    new_properties.push(p);
                    None
                }
            })
            .collect::<Vec<MetricProperty>>();

        Ok(Metric {
            name: pm.name.clone().ok_or(SpbError::InvalidMetric)?,
            alias: pm.alias,
            timestamp: pm.timestamp.unwrap_or(now_milliseconds()),
            datatype: pm.datatype.ok_or(SpbError::InvalidMetric)?,
            is_null: pm.is_null.unwrap_or(false),
            stale: false,
            value,
            in_property: in_property,
            properties: new_properties,
        })
    }
}

#[derive(Clone, Serialize)]
pub struct DataMetric {
    pub name: Option<String>,
    pub alias: Option<u64>,
    pub timestamp: u64,
    pub datatype: Option<u32>,
    pub is_null: Option<bool>,
    pub value: Option<Value>,
    pub properties: Vec<Property>,
}

impl TryFrom<&payload::Metric> for DataMetric {
    type Error = SpbError;

    fn try_from(pm: &payload::Metric) -> Result<Self, Self::Error> {
        if pm.name.is_none() && pm.alias.is_none() {
            return Err(SpbError::InvalidMetric);
        }
        Ok(DataMetric {
            name: pm.name.clone(),
            alias: pm.alias,
            timestamp: pm.timestamp.unwrap_or(now_milliseconds()),
            datatype: pm.datatype,
            is_null: pm.is_null,
            value: if let Some(ref v) = pm.value {
                Some(Value::try_from((v.clone(), pm.datatype))?)
            } else {
                None
            },
            properties: vec![],
        })
    }
}

#[derive(Clone, Serialize)]
pub struct Property {
    pub keys: Vec<String>,
    pub values: Vec<(Option<u32>, Option<bool>, Option<Value>)>,
}

impl TryFrom<payload::PropertySet> for Property {
    type Error = SpbError;

    fn try_from(prop: payload::PropertySet) -> Result<Self, Self::Error> {
        let values = prop
            .values
            .into_iter()
            .map(|pv| {
                let tp = pv.r#type;
                if let Some(value) = pv.value {
                    let value = Value::try_from((value, tp))?;
                    Ok((tp, pv.is_null, Some(value)))
                } else {
                    Ok((tp, pv.is_null, None))
                }
            })
            .collect::<Result<Vec<(Option<u32>, Option<bool>, Option<Value>)>, SpbError>>()?;
        if prop.keys.len() != values.len() {
            return Err(SpbError::InvalidPropertySet);
        }
        for value in values.iter() {
            if value.0.is_none() && value.2.is_none() {
                return Err(SpbError::InvalidPropertySet);
            }
        }

        Ok(Property {
            keys: prop.keys,
            values,
        })
    }
}

impl TryFrom<(payload::property_value::Value, Option<u32>)> for Value {
    type Error = SpbError;

    fn try_from(
        (value, tp): (payload::property_value::Value, Option<u32>),
    ) -> Result<Self, Self::Error> {
        use payload::property_value::Value as PV;
        match value {
            PV::IntValue(v) => match tp {
                Some(1) => Ok(Value::Int8(v as i8)),
                Some(2) => Ok(Value::Int16(v as i16)),
                Some(3) => Ok(Value::Int32(v as i32)),
                Some(5) => Ok(Value::UInt8(v as u8)),
                Some(6) => Ok(Value::UInt16(v as u16)),
                Some(7) => Ok(Value::UInt32(v)),
                None => Ok(Value::UInt32(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            PV::LongValue(v) => match tp {
                Some(4) => Ok(Value::Int64(v as i64)),
                Some(8) => Ok(Value::UInt64(v)),
                Some(11) => Ok(Value::DateTime(v)),
                None => Ok(Value::UInt64(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            PV::FloatValue(v) => match tp {
                Some(9) => Ok(Value::Float(v)),
                None => Ok(Value::Float(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            PV::DoubleValue(v) => match tp {
                Some(10) => Ok(Value::Double(v)),
                None => Ok(Value::Double(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            PV::BooleanValue(v) => match tp {
                Some(11) => Ok(Value::Boolean(v)),
                None => Ok(Value::Boolean(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            PV::StringValue(v) => match tp {
                Some(12) => Ok(Value::String(v)),
                Some(14) => Ok(Value::Text(v)),
                Some(15) => Ok(Value::UUID(v)),
                None => Ok(Value::String(v)),
                _ => Err(SpbError::InvalidDataType),
            },
            _ => Err(SpbError::InvalidDataType),
        }
    }
}
