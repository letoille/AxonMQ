use std::collections::HashMap;

use serde::Serialize;

use super::super::error::SpbError;
use super::metric::{DataMetric, Metric, MetricProperty, Property};

#[derive(Clone, Serialize)]
pub struct Template {
    pub name: String,
    pub version: Option<String>,

    pub metrics: HashMap<String, Metric>,
    pub alias: HashMap<u64, String>,

    pub in_properties: Vec<MetricProperty>,
    pub properties: Vec<Property>,
}

#[derive(Clone, Serialize)]
pub struct TemplateInstance {
    pub name: String,
    pub version: Option<String>,

    pub metrics: HashMap<String, Metric>,
    pub alias: HashMap<u64, String>,

    pub in_properties: Vec<MetricProperty>,
    pub properties: Vec<Property>,
}

impl TemplateInstance {
    pub fn update(&mut self, data_instance: &TemplateDataInstance) -> Result<(), SpbError> {
        for data_metric in data_instance.metrics.iter() {
            let metric = if let Some(name) = &data_metric.name {
                self.metrics.get_mut(name)
            } else if let Some(alias) = &data_metric.alias {
                let metric_name = match self.alias.get(alias) {
                    Some(n) => n,
                    None => return Err(SpbError::MetricNotFound),
                };
                self.metrics.get_mut(metric_name)
            } else {
                return Err(SpbError::MetricNotFound);
            };

            if let Some(m) = metric {
                m.update(data_metric.clone())?;
            } else {
                return Err(SpbError::MetricNotFound);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Serialize)]
pub struct TemplateDataInstance {
    pub template_ref: String,
    pub version: Option<String>,
    pub metrics: Vec<DataMetric>,
}

impl TemplateDataInstance {
    pub fn validate_instance(&self, definition: &Template) -> Result<(), SpbError> {
        if self.version.is_some()
            && definition.version.is_some()
            && self.version.as_ref() != definition.version.as_ref()
        {
            return Err(SpbError::TemplateVersionMismatch);
        }

        for metric in self.metrics.iter() {
            if let Some(name) = &metric.name {
                definition
                    .metrics
                    .get(name)
                    .ok_or(SpbError::InvalidTemplateInstance)?
            } else if let Some(alias) = &metric.alias {
                let template_metric_name = definition
                    .alias
                    .get(alias)
                    .ok_or(SpbError::InvalidTemplateInstance)?;
                definition
                    .metrics
                    .get(template_metric_name)
                    .ok_or(SpbError::InvalidTemplateInstance)?
            } else {
                return Err(SpbError::InvalidTemplateInstance);
            };
        }

        Ok(())
    }
}
