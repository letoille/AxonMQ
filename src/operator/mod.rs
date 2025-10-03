mod command;
mod error;
pub mod helper;
mod matcher;
pub mod sink;
mod trie;
mod utils;

pub struct Operator {
    matcher: matcher::Matcher,
}

impl Operator {
    pub fn new() -> Self {
        Operator {
            matcher: matcher::Matcher::new(),
        }
    }

    pub fn run(&mut self) {
        self.matcher.run();
    }

    pub fn matcher_helper(&self) -> helper::Helper {
        self.matcher.helper()
    }
}
