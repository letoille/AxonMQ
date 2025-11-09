use std::collections::HashMap;

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

use crate::error::AxonError;

use crate::service::sparkplug_b::model::{metric::Metric, template::Template};

#[derive(Clone, Serialize)]
pub struct GetNodeResponse {
    pub node_id: String,
    pub online: bool,
    pub timestamp: u64,

    pub setting: Vec<Metric>,
    pub metrics: Vec<Metric>,

    pub templates: HashMap<String, Template>,
}

#[derive(Clone, Serialize)]
pub struct GetDeviceResponse {
    pub device: String,
    pub online: bool,
    pub timestamp: u64,

    pub setting: Vec<Metric>,
    pub metrics: Vec<Metric>,
}

pub enum InMessage {
    GetGroups {
        group: Option<String>,
        resp: oneshot::Sender<Result<Vec<String>, AxonError>>,
    },
    GetNodes {
        group_id: String,
        node_id: Option<String>,
        resp: oneshot::Sender<Result<Vec<GetNodeResponse>, AxonError>>,
    },
    GetDevice {
        group_id: String,
        node_id: String,
        device: Option<String>,
        resp: oneshot::Sender<Result<Vec<GetDeviceResponse>, AxonError>>,
    },
}

#[derive(Clone)]
pub struct InHelper {
    tx: mpsc::Sender<InMessage>,
}

impl InHelper {
    pub fn new(tx: mpsc::Sender<InMessage>) -> Self {
        InHelper { tx }
    }

    pub async fn get_groups(&self, group: Option<String>) -> Result<Vec<String>, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::GetGroups {
            group,
            resp: resp_tx,
        };

        timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??
    }

    pub async fn get_nodes(
        &self,
        group_id: String,
        node_id: Option<String>,
    ) -> Result<Vec<GetNodeResponse>, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::GetNodes {
            group_id,
            node_id,
            resp: resp_tx,
        };

        timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??
    }

    pub async fn get_devices(
        &self,
        group_id: String,
        node_id: String,
        device: Option<String>,
    ) -> Result<Vec<GetDeviceResponse>, AxonError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = InMessage::GetDevice {
            group_id,
            node_id,
            device,
            resp: resp_tx,
        };

        timeout(Duration::from_secs(5), {
            self.tx.send(msg).await?;
            resp_rx
        })
        .await??
    }
}
