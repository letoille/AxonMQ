pub mod local;

use dyn_clone::DynClone;

use crate::processor::message::Message;

pub trait Sink: Send + Sync + DynClone {
    fn deliver(&self, message: Message, persist: bool);
}

dyn_clone::clone_trait_object!(Sink);

#[derive(Clone)]
pub struct DefaultSink;

impl DefaultSink {
    pub fn new() -> Box<Self> {
        Box::new(DefaultSink {})
    }
}

impl Sink for DefaultSink {
    fn deliver(&self, _message: Message, _persist: bool) {}
}
