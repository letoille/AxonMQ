use std::collections::HashMap;

use super::node::Node;

pub struct Group {
    pub nodes: HashMap<String, Node>,
}
