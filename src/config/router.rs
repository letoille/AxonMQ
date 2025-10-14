use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Router {
    pub topic: String,
    pub client_id: Option<String>,
    pub chain: Vec<String>,
}
