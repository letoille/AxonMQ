use crate::processor::ProcessorInstance;

use super::trie::ClientId;

#[derive(Clone)]
pub struct ProcessorChain {
    pub name: String,
    pub processors: Vec<ProcessorInstance>,
    pub delivery: bool,
}

#[derive(Clone)]
pub struct Chain {
    pub topic_filter: String,
    pub client_id: Option<String>,

    pub chains: Vec<String>,
}

impl ClientId for Chain {
    fn client_id(&self) -> &str {
        &self.topic_filter
    }
}

impl PartialEq for Chain {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.topic_filter == other.topic_filter
    }
}

impl Eq for Chain {}

impl std::hash::Hash for Chain {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(ref client_id) = self.client_id {
            (client_id.to_string() + &self.topic_filter).hash(state);
        } else {
            self.topic_filter.hash(state);
        }
    }
}
