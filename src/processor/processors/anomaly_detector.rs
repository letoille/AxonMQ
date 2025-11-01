use std::any::Any;
use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use minijinja::{Environment, Value, context};
use serde_json::json;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use super::super::{
    Processor,
    config::{AnomalyStrategy, ProcessorConfig},
    error::ProcessorError,
    message::Message,
};

#[derive(Clone, Debug)]
struct MovingAverageState {
    window: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

#[derive(Clone, Debug)]
struct EwmaState {
    mean: f64,
    variance: f64,
    count: u64,
}

#[derive(Clone, Debug)]
enum SeriesState {
    MovingAverage(MovingAverageState),
    Ewma(EwmaState),
}

#[derive(Clone)]
pub struct AnomalyDetectorProcessor {
    id: Uuid,
    env: Arc<Environment<'static>>,
    value_selector: String,
    series_id: String,
    strategy: AnomalyStrategy,
    state: Arc<DashMap<String, SeriesState>>,
}

impl AnomalyDetectorProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
        env: Arc<Environment<'static>>,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::AnomalyDetector {
            value_selector,
            series_id,
            strategy,
        } = config
        {
            Ok(Box::new(AnomalyDetectorProcessor {
                id,
                env,
                value_selector,
                series_id,
                strategy,
                state: Arc::new(DashMap::new()),
            }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "Invalid configuration for AnomalyDetectorProcessor".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Processor for AnomalyDetectorProcessor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(skip(self, message), fields(id = %self.id))]
    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        let payload_json: Value = match serde_json::from_slice(&message.payload) {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "Failed to parse payload as JSON, skipping anomaly detection.");
                return Ok(Some(message));
            }
        };

        let ctx = context! {
            topic => message.topic.clone(),
            client_id => message.client_id.clone(),
            payload => payload_json,
            metadata => message.metadata.clone(),
        };

        let series_key = match self.env.render_str(&self.series_id, ctx.clone()) {
            Ok(key) => key,
            Err(e) => {
                warn!(error = %e, "Failed to render series_id template, cannot perform stateful anomaly detection.");
                return Ok(Some(message));
            }
        };

        let value_template = format!("{{{{ {} }}}}", self.value_selector);
        let rendered_value_str = match self.env.render_str(&value_template, ctx) {
            Ok(s) if !s.is_empty() => s,
            _ => return Ok(Some(message)),
        };

        let value = match rendered_value_str.parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                debug!(selector = %self.value_selector, value = %rendered_value_str, "Selected value is not a valid f64.");
                return Ok(Some(message));
            }
        };

        let mut anomaly_report = None;

        match &self.strategy {
            AnomalyStrategy::Threshold { min, max } => {
                if value < *min || value > *max {
                    anomaly_report = Some(json!({
                        "strategy": "threshold",
                        "value": value,
                        "threshold": { "min": min, "max": max },
                        "details": format!("Value {} is outside the fixed range [{}, {}]", value, min, max)
                    }));
                }
            }
            AnomalyStrategy::MovingAverage {
                window_size,
                deviation_factor,
            } => {
                let mut entry = self.state.entry(series_key.clone()).or_insert_with(|| {
                    SeriesState::MovingAverage(MovingAverageState {
                        window: VecDeque::with_capacity(*window_size),
                        sum: 0.0,
                        sum_sq: 0.0,
                    })
                });

                if let SeriesState::MovingAverage(state) = entry.value_mut() {
                    let n = state.window.len() as f64;
                    if n >= (*window_size as f64) {
                        let mean = state.sum / n;
                        let variance = (state.sum_sq - state.sum * state.sum / n) / n;
                        let std_dev = if variance > 0.0 { variance.sqrt() } else { 0.0 };

                        let threshold = std_dev * deviation_factor;
                        let lower_bound = mean - threshold;
                        let upper_bound = mean + threshold;

                        if value < lower_bound || value > upper_bound {
                            anomaly_report = Some(json!({
                                "strategy": "moving_average",
                                "value": value,
                                "mean": mean,
                                "std_dev": std_dev,
                                "threshold": { "lower": lower_bound, "upper": upper_bound },
                                "details": format!("Value {} is outside the {}σ range [{}, {}]", value, deviation_factor, lower_bound, upper_bound)
                            }));
                        }
                    }

                    // Update window and running sums
                    if state.window.len() >= *window_size {
                        if let Some(old_val) = state.window.pop_front() {
                            state.sum -= old_val;
                            state.sum_sq -= old_val.powi(2);
                        }
                    }
                    state.window.push_back(value);
                    state.sum += value;
                    state.sum_sq += value.powi(2);
                }
            }
            AnomalyStrategy::Ewma {
                alpha,
                deviation_factor,
            } => {
                let mut entry = self.state.entry(series_key.clone()).or_insert_with(|| {
                    SeriesState::Ewma(EwmaState {
                        mean: 0.0,
                        variance: 0.0,
                        count: 0,
                    })
                });

                if let SeriesState::Ewma(state) = entry.value_mut() {
                    if state.count == 0 {
                        state.mean = value;
                    } else {
                        let diff = value - state.mean;
                        let incr = *alpha * diff;
                        state.mean += incr;
                        state.variance = (1.0 - *alpha) * (state.variance + diff * incr);
                    }
                    state.count += 1;

                    if state.count > 1 {
                        let std_dev = state.variance.sqrt();
                        let threshold = std_dev * deviation_factor;
                        if (value - state.mean).abs() > threshold {
                            anomaly_report = Some(
                                json!({ "strategy": "ewma", "value": value, "mean": state.mean, "std_dev": std_dev }),
                            );
                        }
                    }
                }
            }
        }

        if let Some(report) = anomaly_report {
            warn!(
                id = %self.id,
                series_key = %series_key,
                "Anomaly detected: {}",
                report.to_string()
            );
        }

        Ok(Some(message))
    }
}
