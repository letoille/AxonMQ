use std::collections::HashMap;

use super::super::error::SpbError;
use super::super::in_helper::GetDeviceResponse;
use super::metric::{DataMetric, Metric};
use super::node::Node;
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
}
