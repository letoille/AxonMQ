use std::collections::HashMap;

use crate::utils::time::now_milliseconds;

use super::super::error::SpbError;
use super::super::in_helper::{GetNodeResponse, KV};
use super::super::proto;
use super::device::Device;
use super::metric::{DataMetric, Metric};
use super::template::{Template, TemplateDataInstance, TemplateInstance};
use super::value::Value;

pub struct Node {
    pub node_id: String,
    pub online: bool,

    pub bd_seq: u64,
    pub timestamp: u64,

    pub metrics: HashMap<String, Metric>,
    pub aliases: HashMap<u64, String>,
    pub templates: HashMap<String, Template>,

    pub devices: HashMap<String, Device>,
}

impl From<&Node> for GetNodeResponse {
    fn from(node: &Node) -> Self {
        let metrics: Vec<Metric> = node.metrics.values().cloned().collect();
        let (setting, metrics) = metrics
            .into_iter()
            .partition::<Vec<Metric>, _>(|m| m.is_setting());

        GetNodeResponse {
            node_id: node.node_id.clone(),
            online: node.online,
            timestamp: node.timestamp,
            setting,
            metrics,
            templates: node.templates.clone(),
        }
    }
}

impl Node {
    pub fn new(node_id: &str, timestamp: u64, bd_seq: u64) -> Self {
        Self {
            node_id: node_id.to_string(),
            online: false,
            bd_seq,
            timestamp,
            devices: HashMap::new(),
            metrics: HashMap::new(),
            aliases: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    pub fn death(&mut self, timestamp: u64, bd_seq: u64) -> Result<(), SpbError> {
        if timestamp < self.timestamp {
            return Err(SpbError::Exceeded);
        }

        if bd_seq != self.bd_seq {
            return Err(SpbError::NDeathNotMatch);
        }

        self.online = false;
        self.timestamp = timestamp;

        for (_, device) in self.devices.iter_mut() {
            let _ = device.death(timestamp);
        }

        for (_name, metric) in self.metrics.iter_mut() {
            metric.stale = true;
        }
        Ok(())
    }

    pub fn update_metrics(
        &mut self,
        timestamp: u64,
        metrics: Vec<DataMetric>,
    ) -> Result<(), SpbError> {
        if self.online == false {
            return Err(SpbError::NodeNotBirth);
        }

        if timestamp < self.timestamp {
            return Err(SpbError::Exceeded);
        }

        for metric in metrics.into_iter() {
            if let Some(Value::Template(_)) = &metric.value {
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

    pub fn validate_template_instance(
        &self,
        instance: &TemplateDataInstance,
    ) -> Result<(), SpbError> {
        let define = match self.templates.get(&instance.template_ref) {
            Some(d) => d,
            None => return Err(SpbError::TemplateNotFound),
        };

        instance.validate_instance(define)
    }

    pub fn generate_template_instance(
        &self,
        data_instance: &TemplateDataInstance,
    ) -> TemplateInstance {
        match self.templates.get(&data_instance.template_ref) {
            Some(d) => {
                let mut instance = TemplateInstance {
                    name: data_instance.template_ref.clone(),
                    version: data_instance.version.clone(),
                    metrics: d.metrics.clone(),
                    alias: d.alias.clone(),
                    in_properties: d.in_properties.clone(),
                    properties: d.properties.clone(),
                };

                for data_metric in data_instance.metrics.iter() {
                    let metric_name = if let Some(name) = &data_metric.name {
                        name.clone()
                    } else if let Some(alias) = &data_metric.alias {
                        let template_metric_name =
                            d.alias.get(alias).expect("Template alias must exist");
                        template_metric_name.clone()
                    } else {
                        unreachable!("Metric must have name or alias");
                    };

                    if let Some(metric) = instance.metrics.get_mut(&metric_name) {
                        metric.update(data_metric.clone()).unwrap();
                    }
                }

                instance
            }
            None => unreachable!("Template must exist"),
        }
    }

    pub fn birth_with_metrics(
        &mut self,
        timestamp: u64,
        metrics: HashMap<String, Metric>,
    ) -> Result<(), SpbError> {
        if timestamp < self.timestamp {
            return Err(SpbError::Exceeded);
        }

        let mut local_templates_define: HashMap<String, Template> = HashMap::new();
        let mut local_templates_instance: HashMap<String, TemplateDataInstance> = HashMap::new();

        for metric in metrics.values() {
            if let Some(Value::Template(ref template)) = metric.value {
                local_templates_define.insert(metric.name.clone(), template.clone());
            } else if let Some(Value::TemplateDataInstance(ref instance)) = metric.value {
                local_templates_instance.insert(instance.template_ref.clone(), instance.clone());
            }
        }

        for (name, instance) in local_templates_instance.into_iter() {
            let define = match local_templates_define.get(&name) {
                Some(d) => d,
                None => match self.templates.get(&name) {
                    Some(d) => d,
                    None => return Err(SpbError::TemplateNotFound),
                },
            };

            instance.validate_instance(define)?;
        }

        self.templates = local_templates_define;

        for mut metric in metrics.into_values().into_iter() {
            if let Some(Value::Template(_)) = &metric.value {
                continue;
            } else if let Some(Value::TemplateDataInstance(data_instance)) = &metric.value {
                metric.value = Some(Value::TemplateInstance(
                    self.generate_template_instance(data_instance),
                ));
            }

            if let Some(alias) = metric.alias {
                self.aliases.insert(alias, metric.name.clone());
            }

            self.metrics.insert(metric.name.clone(), metric);
        }

        self.timestamp = timestamp;
        self.online = true;

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
