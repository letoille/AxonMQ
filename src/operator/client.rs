use crate::mqtt::QoS;

use super::command::OutSender;
use super::trie::ClientId;

#[derive(Clone)]
pub struct ClientInfo<T: OutSender + Default> {
    pub(crate) client_id: String,

    pub(crate) share_group: Option<String>,
    pub(crate) topic: String,

    pub(crate) qos: QoS,

    pub(crate) no_local: bool,
    pub(crate) store: bool,

    pub(crate) sender: T,
}

impl<T: OutSender + Default> std::cmp::PartialEq for ClientInfo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.share_group == other.share_group
    }
}

impl<T: OutSender + Default> std::cmp::Eq for ClientInfo<T> {}

impl<T: OutSender + Default> std::hash::Hash for ClientInfo<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.client_id.hash(state);
    }
}

impl<T: OutSender + Default> ClientId for ClientInfo<T> {
    fn client_id(&self) -> &str {
        &self.client_id
    }
}

impl<T: OutSender + Default> ClientInfo<T> {
    pub fn filter(&self, client_id: &str) -> bool {
        !(self.no_local && self.client_id == client_id)
    }

    pub fn default(client_id: String, share_group: Option<String>) -> Self {
        ClientInfo {
            client_id,
            share_group,
            topic: String::new(),
            no_local: false,
            sender: T::default(),
            qos: QoS::AtMostOnce,
            store: false,
        }
    }

    pub fn new(
        client_id: String,
        share_group: Option<String>,
        topic: String,
        no_local: bool,
        sender: T,
        qos: QoS,
        store: bool,
    ) -> Self {
        ClientInfo {
            client_id,
            share_group,
            topic,
            no_local,
            sender,
            qos,
            store,
        }
    }
}
