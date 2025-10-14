use serde::Deserialize;

use crate::processor::config::ProcessorConfig;

#[derive(Debug, Deserialize)]
pub struct Processor {
    pub uuid: String,
    pub config: ProcessorConfig,
}
