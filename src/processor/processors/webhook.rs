use std::any::Any;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use minijinja::{Environment, context};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method};
use serde_json::Value as JsonValue;
use tokio::sync::Semaphore;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::processor::message::{MetadataKey, MetadataValue};

use super::super::{Processor, config::ProcessorConfig, error::ProcessorError, message::Message};

const DEFAULT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MAX_CONCURRENCY: usize = 100;

#[derive(Clone)]
pub struct WebhookProcessor {
    id: Uuid,
    client: Client,
    semaphore: Arc<Semaphore>,
    url: String,
    method: Method,
    headers: HeaderMap,
    env: Arc<Environment<'static>>,
    body_template: Option<String>,
}

impl WebhookProcessor {
    pub fn new_with_id(
        id: Uuid,
        config: ProcessorConfig,
        env: Arc<Environment<'static>>,
    ) -> Result<Box<dyn Processor>, ProcessorError> {
        if let ProcessorConfig::WebHook {
            url,
            method,
            headers,
            body_template,
            timeout_ms,
            max_concurrency,
        } = config
        {
            let timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_SECS * 1000);
            let client = Client::builder()
                .timeout(Duration::from_millis(timeout))
                .build()
                .map_err(|e| {
                    ProcessorError::InvalidConfiguration(format!(
                        "Failed to build HTTP client: {}",
                        e
                    ))
                })?;

            let method = method
                .map(|m| Method::from_str(&m.to_uppercase()).unwrap_or(Method::POST))
                .unwrap_or(Method::POST);

            let mut header_map = HeaderMap::new();
            for (k, v) in headers {
                let header_name = HeaderName::from_str(&k).map_err(|e| {
                    ProcessorError::InvalidConfiguration(format!("Invalid header name: {}", e))
                })?;
                let header_value = HeaderValue::from_str(&v).map_err(|e| {
                    ProcessorError::InvalidConfiguration(format!("Invalid header value: {}", e))
                })?;
                header_map.insert(header_name, header_value);
            }

            let concurrency = max_concurrency.unwrap_or(DEFAULT_MAX_CONCURRENCY);
            let semaphore = Arc::new(Semaphore::new(concurrency));

            Ok(Box::new(WebhookProcessor {
                id,
                client,
                semaphore,
                url,
                method,
                headers: header_map,
                env,
                body_template,
            }))
        } else {
            Err(ProcessorError::InvalidConfiguration(
                "Invalid configuration for WebhookProcessor".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Processor for WebhookProcessor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    #[instrument(skip(self, message), fields(id = %self.id))]
    async fn on_message(&self, mut message: Message) -> Result<Option<Message>, ProcessorError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("Semaphore should not be closed");

        let body = match &self.body_template {
            Some(template) => {
                let payload_json = if let Some(MetadataValue::Json(val)) = message
                    .metadata
                    .get(MetadataKey::ParsedPayloadJson.as_str())
                {
                    val.clone()
                } else {
                    let parsed_val: JsonValue = match serde_json::from_slice(&message.payload) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!(error = %e, "Failed to parse payload as JSON for webhook template, passing through.");
                            return Ok(Some(message));
                        }
                    };
                    message.metadata.insert(
                        MetadataKey::ParsedPayloadJson.as_str().to_string(),
                        MetadataValue::Json(parsed_val.clone()),
                    );
                    parsed_val
                };

                let ctx = context! {
                    topic => message.topic.clone(),
                    client_id => message.client_id.clone(),
                    qos => message.qos as u8,
                    retain => message.retain,
                    payload => payload_json,
                    metadata => message.metadata.clone(),
                };

                self.env
                    .render_str(template, ctx)
                    .map_err(|e| {
                        warn!("Failed to render webhook template: {}", e);
                        ProcessorError::TemplateError("Template rendering failed".to_string())
                    })?
                    .into()
            }
            None => message.payload.clone(),
        };

        let start_time = coarsetime::Instant::now();

        let res = self
            .client
            .request(self.method.clone(), &self.url)
            .headers(self.headers.clone())
            .body(body)
            .send()
            .await;

        let duration = start_time.elapsed();

        match res {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    tracing::trace!(url = %self.url, ?duration, status = %status, "request sent");
                } else {
                    tracing::warn!(url = %self.url, ?duration, status = %status, "request failed");
                }
            }
            Err(e) => {
                tracing::error!(url = %self.url, ?duration, error = %e, "Failed to send webhook request");
            }
        }

        Ok(Some(message))
    }
}
