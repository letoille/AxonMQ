use anyhow::{Context, Result};
use serde::Deserialize;
use toml;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
    pub mqtt: MqttConfig,
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct MqttConfig {
    pub listener: MqttListenerConfig,
    pub settings: MqttSettings,
}

#[derive(Debug, Deserialize)]
pub struct MqttListenerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct MqttSettings {
    pub max_topic_length: usize,
    pub session_expiry_interval: u32,
    pub keep_alive: u16,
    pub max_receive_queue: u16,
    pub max_packet_size: u32,
    pub resend_interval: u64,
    pub max_store_msgs_per_client: usize,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("failed to read config file")?;
        let raw: Config = toml::from_str(&content).context("failed to parse config file")?;

        Ok(raw)
    }
}
