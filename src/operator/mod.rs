mod client;
mod command;
mod error;
pub mod helper;
pub mod out;
mod retain_trie;
mod router;
mod trie;
mod utils;

use out::mqtt::MqttOutSender;
use router::Router;

use crate::mqtt::helper::BrokerHelper;

pub struct Operator {
    mqtt_router: Router<MqttOutSender>,
    broker_helper: BrokerHelper,
}

impl Operator {
    pub fn new(broker_helper: BrokerHelper) -> Self {
        Operator {
            mqtt_router: Router::new(),
            broker_helper,
        }
    }

    pub fn run(&mut self) {
        self.mqtt_router.run(self.broker_helper.clone());
    }

    pub fn mqtt_helper(&self) -> helper::Helper<MqttOutSender> {
        self.mqtt_router.helper()
    }
}
