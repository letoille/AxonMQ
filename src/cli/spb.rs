use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

use crate::commands::{GetResource, SetResource, Spb, SpbCommands};
use crate::client::{make_request, make_put_request};

#[derive(Serialize)]
struct CliKV {
    name: String,
    value: Value,
}

fn parse_metrics(metrics: &[String]) -> Result<Vec<CliKV>> {
    let mut kvs = Vec::new();
    for m in metrics {
        let parts: Vec<&str> = m.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid metric format: {}. Expected key=value", m));
        }
        let key = parts[0].to_string();
        let value_str = parts[1];

        let value: Value;
        if value_str.starts_with('"') && value_str.ends_with('"') {
            // Explicitly a string
            value = Value::String(value_str.trim_matches('"').to_string());
        } else {
            // Try to parse as JSON (number, bool, etc.), fallback to string
            value = serde_json::from_str(value_str).unwrap_or_else(|_| Value::String(value_str.to_string()));
        }

        kvs.push(CliKV {
            name: key,
            value,
        });
    }
    Ok(kvs)
}

pub async fn handle_spb_command(spb: Spb, host: &str, client: &Client) -> Result<()> {
    match spb.command {
        SpbCommands::Get(get) => match get.resource {
            GetResource::Groups => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups", host);
                let json = make_request(client, &url).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
            GetResource::Group { group_id } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}", host, group_id);
                let json = make_request(client, &url).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
            GetResource::Nodes { group_id } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}/nodes", host, group_id);
                let json = make_request(client, &url).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
            GetResource::Node { group_id, node_id } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}/nodes/{}", host, group_id, node_id);
                let json = make_request(client, &url).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
            GetResource::Devices { group_id, node_id } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}/nodes/{}/devices", host, group_id, node_id);
                let json = make_request(client, &url).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
            GetResource::Device { group_id, node_id, device_id } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}/nodes/{}/devices/{}", host, group_id, node_id, device_id);
                let json = make_request(client, &url).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
        },
        SpbCommands::Set(set) => match set.resource {
            SetResource::Node { group_id, node_id, metrics } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}/nodes/{}", host, group_id, node_id);
                let kvs = parse_metrics(&metrics)?;
                let json = make_put_request(client, &url, &kvs).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
            SetResource::Device { group_id, node_id, device_id, metrics } => {
                let url = format!("{}/api/v1/services/sparkplug_b/groups/{}/nodes/{}/devices/{}", host, group_id, node_id, device_id);
                let kvs = parse_metrics(&metrics)?;
                let json = make_put_request(client, &url, &kvs).await?;
                let pretty_json = serde_json::to_string_pretty(&json)?;
                println!("{}", pretty_json);
            }
        },
    }
    Ok(())
}
