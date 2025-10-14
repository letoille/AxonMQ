pub mod config;
pub mod error;
pub mod message;
pub mod processors;

use std::any::Any;

use async_trait::async_trait;
use dyn_clone::DynClone;
use uuid::Uuid;

#[async_trait]
pub trait Processor: Send + Sync + DynClone {
    fn id(&self) -> Uuid;
    fn as_any(&self) -> &dyn Any;

    async fn process(
        &self,
        message: message::Message,
    ) -> Result<Option<message::Message>, error::ProcessorError>;
}

dyn_clone::clone_trait_object!(Processor);

#[derive(Clone)]
pub struct ProcessorInstance {
    pub processor: Box<dyn Processor>,
}

impl From<Box<dyn Processor>> for ProcessorInstance {
    fn from(processor: Box<dyn Processor>) -> Self {
        ProcessorInstance { processor }
    }
}
