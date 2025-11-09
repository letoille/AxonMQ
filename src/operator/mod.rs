mod chain;
mod command;
pub mod error;
mod filter;
pub mod helper;
mod matcher;
mod router;
pub mod sink;
mod trie;
mod utils;

use crate::service::sparkplug_b::helper::SparkPlugBApplicationHelper;

pub struct Operator {
    matcher: matcher::Matcher,
    router: router::Router,
}

impl Operator {
    pub async fn new() -> Self {
        let matcher = matcher::Matcher::new();
        let router = router::Router::new(matcher.sender()).await;

        Operator { matcher, router }
    }

    pub fn run(&mut self, sparkplug_helper: Option<SparkPlugBApplicationHelper>) {
        self.matcher.run();
        self.router.run(sparkplug_helper);
    }

    pub fn helper(&self) -> helper::Helper {
        helper::Helper::new(self.matcher.sender(), self.router.sender())
    }
}
