use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Chain {
    pub name: String,
    pub processors: Vec<String>,
    pub delivery: bool,
}
