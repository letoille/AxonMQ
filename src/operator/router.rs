use std::collections::HashMap;
use std::sync::Arc;

use uuid;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{info, trace};
use wasmtime::Engine;

use crate::CONFIG;
use crate::processor::message::Message;

use super::chain::{Chain, ProcessorChain};

use super::command::OperatorCommand;
use super::trie::TopicTrie;

pub struct Router {
    command_rx: Option<mpsc::Receiver<OperatorCommand>>,
    command_tx: mpsc::Sender<OperatorCommand>,

    matcher_sender: mpsc::Sender<OperatorCommand>,

    trie: Option<TopicTrie<Chain>>,
    chains: HashMap<String, ProcessorChain>,

    #[allow(dead_code)]
    engine: Arc<Engine>,
}

impl Router {
    fn create_engine() -> Engine {
        use wasmtime::{Cache, CacheConfig, Config};

        let mut config = Config::new();
        //config.async_support(true);
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.wasm_component_model(true);

        let mut cache_config = CacheConfig::new();
        cache_config.with_cleanup_interval(std::time::Duration::from_secs(24 * 60 * 60)); // 1 day
        cache_config.with_files_total_size_soft_limit(1024 * 1024 * 1024); // 1GB
        cache_config.with_directory("./wasm_cache");
        let cache = Cache::new(cache_config).ok();
        config.cache(cache);

        Engine::new(&config).unwrap_or(Engine::default())
    }

    pub async fn new(matcher_sender: mpsc::Sender<OperatorCommand>) -> Self {
        let (tx, rx) = mpsc::channel(1024);
        let mut chains = HashMap::new();
        let mut trie = TopicTrie::new();
        let engine = Arc::new(Self::create_engine());

        let router_configs = &CONFIG.get().unwrap().router;
        for router in router_configs {
            let chain_names = router.chain.clone();
            let chain = Chain {
                topic_filter: router.topic.clone(),
                client_id: router.client_id.clone(),
                chains: chain_names,
            };
            trie.insert(&router.topic, chain);
        }

        let mut processor_map = HashMap::new();

        for processor in &CONFIG.get().unwrap().processor {
            if processor_map.contains_key(&processor.uuid) {
                continue;
            }

            let uuid = uuid::Uuid::parse_str(&processor.uuid).unwrap();
            let proc = processor
                .config
                .new_processor(uuid, engine.clone())
                .await
                .unwrap();
            processor_map.insert(processor.uuid.clone(), proc);
        }

        let chain_configs = &CONFIG.get().unwrap().chain;
        for chain in chain_configs {
            let processors = chain
                .processors
                .iter()
                .filter_map(|name| processor_map.get(name).cloned())
                .map(Into::into)
                .collect::<Vec<_>>();
            chains.insert(
                chain.name.clone(),
                ProcessorChain {
                    name: chain.name.clone(),
                    processors,
                    delivery: chain.delivery,
                },
            );
        }

        Router {
            command_rx: Some(rx),
            command_tx: tx,
            matcher_sender,
            trie: Some(trie),
            chains,
            engine,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<OperatorCommand> {
        self.command_tx.clone()
    }

    pub fn run(&mut self) {
        let mut command_rx = self.command_rx.take().unwrap();
        let matcher_sender = self.matcher_sender.clone();
        let mut trie = self.trie.take().unwrap();
        let chains = self.chains.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(cmd) = command_rx.recv() => {
                        if let OperatorCommand::Publish{client_id, retain, qos, topic, payload, properties, expiry_at} = cmd {
                            let chains = Self::route(&mut trie, &chains, &topic, &client_id);
                            if let Some(chains) = chains {
                                let msg = Message::new(
                                    client_id.clone(),
                                    topic.clone(),
                                    qos,
                                    retain,
                                    expiry_at,
                                    payload.clone(),
                                    properties.clone(),
                                );

                                Self::chains_process(chains, msg, matcher_sender.clone()).await;
                            } else {
                                matcher_sender.send(OperatorCommand::Publish {
                                    client_id,
                                    retain,
                                    qos,
                                    topic,
                                    payload,
                                    properties,
                                    expiry_at,
                                }).await.ok();
                            }
                        } else {
                            trace!("router received unsupported command: {}", cmd);
                        }
                    }
                }
            }
        });
    }

    fn route(
        trie: &mut TopicTrie<Chain>,
        chains: &HashMap<String, ProcessorChain>,
        topic: &str,
        client_id: &str,
    ) -> Option<Vec<ProcessorChain>> {
        let chains_name = trie
            .find_matches(topic)
            .iter()
            .filter(|chain| {
                if let Some(ref cid) = chain.client_id {
                    cid == client_id
                } else {
                    true
                }
            })
            .flat_map(|chain| chain.chains.clone())
            .collect::<Vec<_>>();

        let chains = chains_name
            .iter()
            .filter_map(|name| chains.get(name).cloned())
            .collect::<Vec<_>>();
        if chains.is_empty() {
            None
        } else {
            Some(chains)
        }
    }

    async fn chains_process(
        chains: Vec<ProcessorChain>,
        message: Message,
        matcher_sender: mpsc::Sender<OperatorCommand>,
    ) {
        let mut set = JoinSet::new();

        for chain in chains {
            let mut msg = message.clone();
            set.spawn(async move {
                info!("processing message with chain {}", chain.processors.len());
                for processor in chain.processors {
                    info!(
                        "processing message with processor {} in chain {}",
                        processor.processor.id(),
                        chain.name
                    );
                    match processor.processor.on_message(msg).await {
                        Ok(Some(m)) => {
                            msg = m;
                        }
                        Ok(None) => {
                            trace!(
                                "processor {} in chain {} dropped the message",
                                processor.processor.id(),
                                chain.name
                            );
                            return None;
                        }
                        Err(e) => {
                            trace!(
                                "processor {} in chain {} failed to process message: {}",
                                processor.processor.id(),
                                chain.name,
                                e
                            );
                            return None;
                        }
                    }
                }

                if chain.delivery { Some(msg) } else { None }
            });
        }

        while let Some(result) = set.join_next().await {
            match result {
                Ok(Some(msg)) => {
                    matcher_sender
                        .send(OperatorCommand::Publish {
                            client_id: msg.client_id,
                            retain: msg.retain,
                            qos: msg.qos,
                            topic: msg.topic,
                            payload: msg.payload,
                            properties: msg.properties,
                            expiry_at: msg.expiry_at,
                        })
                        .await
                        .ok();
                }
                Ok(None) => {}
                Err(e) => {
                    trace!("chain processing task failed: {}", e);
                }
            }
        }
    }
}
