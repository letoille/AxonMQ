mod cmd;
pub mod error;
pub mod helper;
pub mod in_helper;
mod message;
mod model;
mod proto;

use std::collections::HashMap;

use tokio::sync::mpsc;
use tracing::{debug, info, info_span};

use crate::operator::helper::Helper as OperatorHelper;
use crate::service::sparkplug_b::model::device::Device;

use error::SpbError;
use in_helper::{GetDeviceResponse, GetNodeResponse, InHelper, InMessage};
use message::MessageType;
use model::{group::Group, node::Node};

pub struct SparkPlugBApplication {
    rx: Option<mpsc::Receiver<helper::Publish>>,
    in_rx: Option<mpsc::Receiver<InMessage>>,
    helper: helper::SparkPlugBApplicationHelper,
    in_helper: InHelper,
}

impl SparkPlugBApplication {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(128);
        let helper = helper::SparkPlugBApplicationHelper::new(tx.clone());
        let (in_tx, in_rx) = mpsc::channel(16);
        let in_helper = InHelper::new(in_tx);

        SparkPlugBApplication {
            rx: Some(rx),
            in_rx: Some(in_rx),
            helper,
            in_helper,
        }
    }

    pub fn helper(&self) -> helper::SparkPlugBApplicationHelper {
        self.helper.clone()
    }

    pub fn in_helper(&self) -> InHelper {
        self.in_helper.clone()
    }

    pub async fn run(&mut self, operator_helper: OperatorHelper) {
        let rx = self.rx.take().unwrap();
        let in_rx = self.in_rx.take().unwrap();
        let mut groups = HashMap::<String, Group>::new();

        tokio::spawn(async move {
            let mut rx = rx;
            let mut in_rx = in_rx;
            let mut cmd = cmd::Cmd::new();

            loop {
                tokio::select! {
                    Some(mut publish) = rx.recv() => {
                        if let Err(e) = Self::on_message(&mut publish, &mut groups) {
                            if let Some(gn) = publish.gn.as_ref() {
                                debug!("message error: {} for group: {}, node: {}", e, gn.0, gn.1);
                            } else {
                                debug!("message error: {}", e);
                            }
                            match e {
                                SpbError::NodeNotBirth
                                | SpbError::DeviceNotBirth
                                | SpbError::MetricNotFound
                                | SpbError::MetricNotMatch => {
                                    if let Some(gn) = publish.gn {
                                         let (topic, payload) = cmd.node_rebirth(
                                            gn.0,
                                            gn.1,
                                            None,
                                        );
                                        let _ = operator_helper.sparkplug_b_publish(
                                            topic, payload
                                        ).await;
                                    }
                                }
                                SpbError::InvalidTopic => {}
                                _ => {}
                            }
                        }
                    }
                    Some(in_msg) = in_rx.recv() => {
                        Self::in_message(in_msg, &mut groups);
                    }
                }
            }
        });
    }

    fn in_message(msg: InMessage, groups: &mut HashMap<String, Group>) {
        use InMessage::*;
        match msg {
            GetGroups { group, resp } => {
                if let Some(ref group_id) = group {
                    if groups.contains_key(group_id) {
                        let _ = resp.send(Ok(vec![group_id.to_string()]));
                    } else {
                        let _ = resp.send(Err(SpbError::GroupNotFound.into()));
                    }
                } else {
                    let values = groups.keys().cloned().collect::<Vec<_>>();
                    let _ = resp.send(Ok(values));
                }
            }
            GetNodes {
                group_id,
                node_id,
                resp,
            } => {
                if let Some(group) = groups.get(&group_id) {
                    if let Some(ref node_id) = node_id {
                        if let Some(node) = group.nodes.get(node_id) {
                            let response: GetNodeResponse = node.into();
                            let _ = resp.send(Ok(vec![response]));
                        } else {
                            let _ = resp.send(Err(SpbError::NodeNotFound.into()));
                        }
                    } else {
                        let responses = group
                            .nodes
                            .values()
                            .map(|node| {
                                let response: GetNodeResponse = node.into();
                                response
                            })
                            .collect::<Vec<_>>();
                        let _ = resp.send(Ok(responses));
                    }
                } else {
                    let _ = resp.send(Err(SpbError::GroupNotFound.into()));
                }
            }
            GetDevice {
                group_id,
                node_id,
                device,
                resp,
            } => {
                if let Some(group) = groups.get(&group_id) {
                    if let Some(node) = group.nodes.get(&node_id) {
                        if let Some(ref device) = device {
                            if let Some(device) = node.devices.get(device) {
                                let response: GetDeviceResponse = device.into();
                                let _ = resp.send(Ok(vec![response]));
                            } else {
                                let _ = resp.send(Err(SpbError::DeviceNotFound.into()));
                            }
                        } else {
                            let responses = node
                                .devices
                                .values()
                                .map(|device| {
                                    let response: GetDeviceResponse = device.into();
                                    response
                                })
                                .collect::<Vec<_>>();
                            let _ = resp.send(Ok(responses));
                        }
                    } else {
                        let _ = resp.send(Err(SpbError::NodeNotFound.into()));
                    }
                } else {
                    let _ = resp.send(Err(SpbError::GroupNotFound.into()));
                }
            }
        }
    }

    fn on_message(
        publish: &mut helper::Publish,
        groups: &mut HashMap<String, Group>,
    ) -> Result<(), SpbError> {
        use MessageType::*;
        let message = publish.parse()?;
        let span = if let Some(ref device) = message.device_id {
            info_span!("spb_message", group_id = %message.group_id, node_id = %message.node_id, device_id = %device)
        } else {
            info_span!("spb_message", group_id = %message.group_id, node_id = %message.node_id)
        };

        match message.msg {
            NodeBirth {
                seq: _,
                timestamp,
                bd_seq,
                metrics,
            } => {
                if let Some(node) = groups
                    .get(&message.group_id)
                    .and_then(|g| g.nodes.get(&message.node_id))
                {
                    if timestamp < node.timestamp {
                        return Err(SpbError::Exceeded);
                    }
                }

                let mut node = Node::new(&message.node_id, timestamp, bd_seq);
                node.birth_with_metrics(timestamp, metrics)?;

                let group = groups
                    .entry(message.group_id.clone())
                    .or_insert_with(|| Group {
                        nodes: HashMap::new(),
                    });
                group.nodes.insert(message.node_id.clone(), node);
                info!(parent: &span, "Node born");
            }
            NodeDeath { timestamp, bd_seq } => {
                if let Some(node) = groups
                    .get_mut(&message.group_id)
                    .and_then(|g| g.nodes.get_mut(&message.node_id))
                {
                    node.death(timestamp, bd_seq)?;
                    info!(parent: &span, "Node died");
                } else {
                    return Err(SpbError::NodeNotFound);
                }
            }
            NodeData {
                seq: _,
                timestamp,
                metrics,
            } => {
                if let Some(node) = groups
                    .get_mut(&message.group_id)
                    .and_then(|g| g.nodes.get_mut(&message.node_id))
                {
                    node.update_metrics(timestamp, metrics)?;
                } else {
                    return Err(SpbError::NodeNotBirth);
                }
            }
            NodeCommand { .. } => {
                return Err(SpbError::NotSupportCommand);
            }
            DeviceBirth {
                seq: _,
                timestamp,
                metrics,
            } => {
                if let Some(node) = groups
                    .get_mut(&message.group_id)
                    .and_then(|g| g.nodes.get_mut(&message.node_id))
                {
                    if node.online == false {
                        return Err(SpbError::NodeNotBirth);
                    }

                    if let Some(device) = node.devices.get(&message.device_id.clone().unwrap()) {
                        if timestamp < device.timestamp {
                            return Err(SpbError::Exceeded);
                        }
                    }

                    let mut device = Device::new(message.device_id.clone().unwrap(), timestamp);
                    device.birth_with_metrics(node, timestamp, metrics)?;

                    node.devices
                        .insert(message.device_id.clone().unwrap(), device);
                    info!(parent: &span, "Device born");
                } else {
                    return Err(SpbError::NodeNotBirth);
                }
            }
            DeviceDeath { seq: _, timestamp } => {
                if let Some(node) = groups
                    .get_mut(&message.group_id)
                    .and_then(|g| g.nodes.get_mut(&message.node_id))
                {
                    if node.online == false {
                        return Err(SpbError::NodeNotBirth);
                    }
                    if let Some(device) = node.devices.get_mut(&message.device_id.unwrap()) {
                        device.death(timestamp)?;
                        info!(parent: &span, "Device died");
                    } else {
                        return Err(SpbError::DeviceNotBirth);
                    }
                } else {
                    return Err(SpbError::NodeNotFound);
                }
            }
            DeviceData {
                seq: _,
                timestamp,
                metrics,
            } => {
                let device = if let Some(node) = groups
                    .get_mut(&message.group_id)
                    .and_then(|g| g.nodes.get_mut(&message.node_id))
                {
                    if node.online == false {
                        return Err(SpbError::NodeNotBirth);
                    }
                    node.devices.remove(&message.device_id.clone().unwrap())
                } else {
                    return Err(SpbError::NodeNotBirth);
                };

                if let Some(mut device) = device {
                    device.update_metrics(
                        groups
                            .get(&message.group_id)
                            .and_then(|g| g.nodes.get(&message.node_id))
                            .unwrap(),
                        timestamp,
                        metrics,
                    )?;
                    groups
                        .get_mut(&message.group_id)
                        .and_then(|g| g.nodes.get_mut(&message.node_id))
                        .ok_or(SpbError::NodeNotBirth)?
                        .devices
                        .insert(message.device_id.unwrap(), device);
                } else {
                    return Err(SpbError::DeviceNotBirth);
                }
            }
            DeviceCommand { .. } => {
                return Err(SpbError::NotSupportCommand);
            }
        }

        Ok(())
    }
}
