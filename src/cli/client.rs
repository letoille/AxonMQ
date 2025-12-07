use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

pub async fn make_request(client: &Client, url: &str) -> Result<Value> {
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("Cannot connect to AxonMQ"));
    }
    let json: Value = response.json().await?;
    Ok(json)
}

pub async fn make_put_request<T: Serialize>(client: &Client, url: &str, body: &T) -> Result<Value> {
    let response = client.put(url).json(body).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("Cannot connect to AxonMQ"));
    }
    let json: Value = response.json().await?;
    Ok(json)
}
