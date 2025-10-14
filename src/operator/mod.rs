mod chain;
mod command;
pub mod error;
pub mod helper;
mod matcher;
mod router;
pub mod sink;
mod trie;
mod utils;

pub struct Operator {
    matcher: matcher::Matcher,
    router: router::Router,
}

impl Operator {
    pub fn new() -> Self {
        let matcher = matcher::Matcher::new();
        let router = router::Router::new(matcher.sender());

        Operator { matcher, router }
    }

    pub fn run(&mut self) {
        self.matcher.run();
        self.router.run();
    }

    pub fn matcher_helper(&self) -> helper::Helper {
        helper::Helper::new(self.matcher.sender(), self.router.sender())
    }
}
