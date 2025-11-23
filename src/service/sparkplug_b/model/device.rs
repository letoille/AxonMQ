use std::collections::HashMap;

use crate::utils::time::now_milliseconds;

use super::super::error::SpbError;
use super::super::in_helper::{GetDeviceResponse, KV};
use super::super::proto;
use super::metric::{DataMetric, Metric};
use super::node::Node;
use super::template::TemplateDataInstance;
use super::value::Value;

pub struct Device {
    pub device: String,
    pub online: bool,
    pub timestamp: u64,

    pub metrics: HashMap<String, Metric>,
    pub aliases: HashMap<u64, String>,
}

impl From<&Device> for GetDeviceResponse {
    fn from(device: &Device) -> Self {
        let metrics: Vec<Metric> = device.metrics.values().cloned().collect();
        let (setting, metrics) = metrics
            .into_iter()
            .partition::<Vec<Metric>, _>(|m| m.is_setting());

        GetDeviceResponse {
            device: device.device.clone(),
            online: device.online,
            timestamp: device.timestamp,
            setting,
            metrics,
        }
    }
}

impl Device {
    pub fn new(device: String, timestamp: u64) -> Self {
        Self {
            device,
            online: true,
            timestamp,
            metrics: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    pub fn death(&mut self, timestamp: u64) -> Result<(), SpbError> {
        if timestamp < self.timestamp {
            return Err(SpbError::Exceeded);
        }

        self.online = false;
        self.timestamp = timestamp;

        for (_name, metric) in self.metrics.iter_mut() {
            metric.stale = true;
        }
        Ok(())
    }

    pub fn update_metrics(
        &mut self,
        node: &Node,
        timestamp: u64,
        metrics: Vec<DataMetric>,
    ) -> Result<(), SpbError> {
        if self.online == false {
            return Err(SpbError::DeviceNotBirth);
        }

        if timestamp < self.timestamp {
            return Err(SpbError::Exceeded);
        }

        for metric in metrics.into_iter() {
            if let Some(Value::TemplateDataInstance(instance)) = &metric.value {
                node.validate_template_instance(instance)?;
            } else if let Some(Value::Template(_)) = &metric.value {
                return Err(SpbError::InvalidMetric);
            }

            let name = if let Some(alias) = metric.alias {
                match self.aliases.get(&alias) {
                    Some(n) => n.clone(),
                    None => return Err(SpbError::MetricNotFound),
                }
            } else {
                metric.name.as_ref().unwrap().clone()
            };

            let find_metric = self
                .metrics
                .get_mut(&name)
                .ok_or(SpbError::MetricNotFound)?;

            find_metric.update(metric)?;
        }

        self.timestamp = timestamp;
        Ok(())
    }

    pub fn birth_with_metrics(
        &mut self,
        node: &Node,
        timestamp: u64,
        metrics: HashMap<String, Metric>,
    ) -> Result<(), crate::service::sparkplug_b::error::SpbError> {
        if timestamp < self.timestamp {
            return Err(SpbError::Exceeded);
        }

        for metric in metrics.values() {
            if let Some(Value::TemplateDataInstance(instance)) = &metric.value {
                node.validate_template_instance(instance)?;
            } else if let Some(Value::Template(_)) = &metric.value {
                return Err(SpbError::InvalidMetric);
            }
        }

        for metric in metrics.into_values().into_iter() {
            if let Some(alias) = metric.alias {
                self.aliases.insert(alias, metric.name.clone());
            }

            self.metrics.insert(metric.name.clone(), metric);
        }

        Ok(())
    }

    fn new_payload(metrics: Vec<proto::payload::Metric>) -> proto::Payload {
        let payload = proto::Payload {
            timestamp: Some(now_milliseconds()),
            metrics,
            seq: Some(0),
            uuid: None,
            body: None,
        };

        payload
    }

    pub fn command(&self, kvs: Vec<KV>) -> (Option<proto::Payload>, Vec<(String, String)>) {
        let mut results = Vec::new();
        let mut metrics = Vec::new();
        let mut errors = false;

        for kv in kvs.into_iter() {
            if let Some(ref template) = kv.template {
                let metric = match self.metrics.get(template) {
                    Some(m) => m,
                    None => {
                        results.push((kv.name.clone(), SpbError::MetricNotFound.to_string()));
                        errors = true;
                        continue;
                    }
                };
                if metric.datatype != 19 {
                    results.push((kv.name.clone(), SpbError::MetricNotMatch.to_string()));
                    errors = true;
                    continue;
                }
                if let Some(Value::TemplateInstance(ref instance)) = metric.value {
                    let l_metric = match instance.metrics.get(&kv.name) {
                        Some(m) => m,
                        None => {
                            results.push((kv.name.clone(), SpbError::MetricNotFound.to_string()));
                            errors = true;
                            continue;
                        }
                    };
                    let value = Value::try_from((kv.value, l_metric.datatype));
                    if let Err(e) = value {
                        results.push((kv.name.clone(), e.to_string()));
                        errors = true;
                        continue;
                    }

                    let l_metric = DataMetric {
                        alias: l_metric.alias,
                        name: if l_metric.alias.is_some() {
                            None
                        } else {
                            Some(l_metric.name.clone())
                        },
                        datatype: Some(l_metric.datatype),
                        timestamp: now_milliseconds(),
                        is_null: None,
                        properties: vec![],
                        value: Some(value.unwrap()),
                    };
                    let data_instance = TemplateDataInstance {
                        template_ref: instance.name.clone(),
                        version: instance.version.clone(),
                        metrics: vec![l_metric],
                    };
                    let template_metric = proto::payload::Metric {
                        name: if metric.alias.is_some() {
                            None
                        } else {
                            Some(metric.name.clone())
                        },
                        alias: metric.alias,
                        datatype: Some(metric.datatype),
                        value: Some(Value::TemplateDataInstance(data_instance).into()),
                        timestamp: Some(now_milliseconds()),
                        properties: None,
                        is_historical: None,
                        is_null: None,
                        is_transient: None,
                        metadata: None,
                    };

                    metrics.push(template_metric);
                    results.push((kv.name.clone(), "success".to_string()));
                } else {
                    results.push((kv.name.clone(), SpbError::MetricNotMatch.to_string()));
                    errors = true;
                    continue;
                }
            } else {
                let metric = match self.metrics.get(&kv.name) {
                    Some(m) => m,
                    None => {
                        results.push((kv.name.clone(), SpbError::MetricNotFound.to_string()));
                        errors = true;
                        continue;
                    }
                };
                let value = Value::try_from((kv.value, metric.datatype));
                if let Err(e) = value {
                    results.push((kv.name.clone(), e.to_string()));
                    errors = true;
                    continue;
                }

                if metric.datatype == 19 {
                    results.push((kv.name.clone(), SpbError::MetricNotMatch.to_string()));
                    errors = true;
                    continue;
                }

                let metric = proto::payload::Metric {
                    name: if metric.alias.is_some() {
                        None
                    } else {
                        Some(metric.name.clone())
                    },
                    alias: metric.alias,
                    datatype: Some(metric.datatype),
                    value: Some(value.unwrap().into()),
                    timestamp: Some(now_milliseconds()),
                    properties: None,
                    is_historical: None,
                    is_null: None,
                    is_transient: None,
                    metadata: None,
                };

                metrics.push(metric);
                results.push((kv.name.clone(), "success".to_string()));
            }
        }

        if errors {
            (None, results)
        } else {
            (Some(Self::new_payload(metrics)), results)
        }
    }
}
