use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, error, info, instrument, trace, warn};
use uuid::Uuid;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable, bindgen};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use super::Processor;
use super::error::ProcessorError;
use super::message::Message;

bindgen!("axonmq-processor" in "wit/processor.wit");
//bindgen!({
//world: "axonmq-processor",
//path: "wit/processor.wit",
//exports: {default: async},
//});

#[derive(Clone)]
pub struct WasmProcessor {
    engine: Arc<Engine>,
    component: Component,
    linker: Linker<WasmProcessorState>,

    uuid: Uuid,

    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    cfg: String,
}

#[allow(dead_code)]
pub struct WasmProcessorState {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,

    pub uuid: Uuid,
}

impl WasiView for WasmProcessorState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

use axonmq::processor::logging;

impl logging::Host for WasmProcessorState {
    #[instrument(name="on_message", skip(self, message, level, target), fields(id=%self.uuid, name=%target))]
    fn log(&mut self, level: logging::LogLevel, target: String, message: String) -> () {
        match level {
            logging::LogLevel::Trace => {
                trace!("{}", message)
            }
            logging::LogLevel::Debug => {
                debug!("{}", message)
            }
            logging::LogLevel::Info => {
                info!("{}", message)
            }
            logging::LogLevel::Warn => {
                warn!("{}", message)
            }
            logging::LogLevel::Error => {
                error!("{}", message)
            }
        }
    }
}

impl WasmProcessor {
    pub async fn new(
        engine: Arc<Engine>,
        wasm_file: String,
        uuid: Uuid,
        cfg: String,
    ) -> Result<Box<dyn Processor>> {
        debug!("Loading WASM Processor from file: {}", wasm_file);

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        AxonmqProcessor::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

        let wasi = WasiCtx::builder().inherit_stdio().inherit_args().build();
        let state = WasmProcessorState {
            wasi_ctx: wasi,
            resource_table: ResourceTable::new(),
            uuid,
        };

        let mut store = Store::new(&engine, state);
        let component = Component::from_file(&engine, wasm_file)?;
        let bindings = AxonmqProcessor::instantiate(&mut store, &component, &linker)?;

        let name = bindings.axonmq_processor_handler().call_name(&mut store)?;
        let version = bindings
            .axonmq_processor_handler()
            .call_version(&mut store)?;
        let description = bindings
            .axonmq_processor_handler()
            .call_description(&mut store)?;
        bindings
            .axonmq_processor_handler()
            .call_set_instance_id(&mut store, &uuid.to_string())?;
        bindings
            .axonmq_processor_handler()
            .call_set_config(&mut store, &cfg)?;

        info!(
            "loaded: {} v{} - {} ({}), cfg: {}",
            name, version, description, uuid, cfg
        );
        Ok(Box::new(WasmProcessor {
            engine,
            component,
            linker,
            name,
            version,
            description,
            uuid,
            cfg,
        }))
    }
}

use self::exports::axonmq::processor::handler;

impl From<Message> for handler::Message {
    fn from(message: Message) -> Self {
        let metadata = message
            .metadata
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    match v {
                        crate::processor::message::MetadataValue::String(s) => {
                            handler::Value::VString(s.clone())
                        }
                        crate::processor::message::MetadataValue::Int(i) => {
                            handler::Value::VInt(*i)
                        }
                        crate::processor::message::MetadataValue::Float(f) => {
                            handler::Value::VFloat(*f)
                        }
                        crate::processor::message::MetadataValue::Bool(b) => {
                            handler::Value::VBool(*b)
                        }
                        crate::processor::message::MetadataValue::Json(j) => {
                            handler::Value::VString(j.to_string())
                        }
                    },
                )
            })
            .collect();
        let properties = message
            .properties
            .iter()
            .filter_map(|p| {
                if let crate::mqtt::protocol::property::Property::UserProperty(v) = p {
                    Some((v.0.clone(), v.1.clone()))
                } else {
                    None
                }
            })
            .collect();

        let msg = handler::Message {
            client_id: message.client_id,
            topic: message.topic,
            qos: match message.qos {
                crate::mqtt::QoS::AtMostOnce => handler::Qos::AtMostOnce,
                crate::mqtt::QoS::AtLeastOnce => handler::Qos::AtLeastOnce,
                crate::mqtt::QoS::ExactlyOnce => handler::Qos::ExactlyOnce,
            },
            retain: message.retain,
            payload: message.payload.to_vec(),
            metadata,
            properties,
        };

        msg
    }
}

impl From<handler::Message> for Message {
    fn from(message: handler::Message) -> Self {
        let qos = match message.qos {
            handler::Qos::AtMostOnce => crate::mqtt::QoS::AtMostOnce,
            handler::Qos::AtLeastOnce => crate::mqtt::QoS::AtLeastOnce,
            handler::Qos::ExactlyOnce => crate::mqtt::QoS::ExactlyOnce,
        };
        let payload = message.payload.into();
        let properties = message
            .properties
            .iter()
            .map(|(k, v)| {
                crate::mqtt::protocol::property::Property::UserProperty((k.clone(), v.clone()))
            })
            .collect();
        let metadata = message
            .metadata
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    match v {
                        handler::Value::VString(s) => {
                            crate::processor::message::MetadataValue::String(s.clone())
                        }
                        handler::Value::VInt(i) => {
                            crate::processor::message::MetadataValue::Int(*i)
                        }
                        handler::Value::VFloat(f) => {
                            crate::processor::message::MetadataValue::Float(*f)
                        }
                        handler::Value::VBool(b) => {
                            crate::processor::message::MetadataValue::Bool(*b)
                        }
                    },
                )
            })
            .collect();

        Message {
            client_id: message.client_id,
            topic: message.topic,
            qos,
            retain: message.retain,
            expiry_at: None,
            payload,
            properties,
            metadata,
        }
    }
}

impl Into<Option<Message>> for handler::MessageResult {
    fn into(self) -> Option<Message> {
        match self {
            handler::MessageResult::Forward(m) => Some(m.into()),
            handler::MessageResult::Drop => None,
        }
    }
}

#[async_trait]
impl Processor for WasmProcessor {
    fn id(&self) -> Uuid {
        self.uuid
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        let wasi = WasiCtx::builder().inherit_stdio().inherit_args().build();
        let state = WasmProcessorState {
            wasi_ctx: wasi,
            resource_table: ResourceTable::new(),
            uuid: self.uuid,
        };
        let mut store = Store::new(&self.engine, state);
        let bindings = AxonmqProcessor::instantiate(&mut store, &self.component, &self.linker)?;

        let result = bindings
            .axonmq_processor_handler()
            .call_on_message(&mut store, &message.into())?
            .map_err(|e| ProcessorError::ProcessorError(e))?;

        Ok(result.into())
    }
}
