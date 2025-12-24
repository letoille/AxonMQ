use std::collections::HashMap;
use std::sync::Arc;

use uuid;

use minijinja::{Environment, Value};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{trace, warn};
use wasmtime::Engine;

use crate::mqtt::protocol::publish::PublishOptions;
use crate::processor::message::Message;
use crate::service::sparkplug_b::helper::SparkPlugBApplicationHelper;
use crate::{CONFIG, get_default_log_dir};

use super::chain::{Chain, ProcessorChain};
use super::filter::MinijinjaFilter;

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
    #[allow(dead_code)]
    minijinja_env: Arc<Environment<'static>>,
}

impl Router {
    fn create_env() -> Environment<'static> {
        let mut env = Environment::new();

        env.add_function("now", || -> Value {
            Value::from(coarsetime::Clock::now_since_epoch().as_millis())
        });

        MinijinjaFilter::register(&mut env);
        env
    }

    fn create_engine() -> Engine {
        use wasmtime::{Cache, CacheConfig, Config};

        let mut config = Config::new();
        //config.async_support(true);
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.wasm_component_model(true);

        let mut cache_config = CacheConfig::new();
        cache_config.with_cleanup_interval(std::time::Duration::from_secs(24 * 60 * 60)); // 1 day
        cache_config.with_files_total_size_soft_limit(1024 * 1024 * 1024); // 1GB
        cache_config
            .with_directory(std::path::Path::new(get_default_log_dir()).join("./wasm_cache"));
        let cache = Cache::new(cache_config).ok();
        config.cache(cache);

        Engine::new(&config).unwrap_or(Engine::default())
    }

    pub async fn new(matcher_sender: mpsc::Sender<OperatorCommand>) -> Self {
        let (tx, rx) = mpsc::channel(1024);
        let mut chains = HashMap::new();
        let mut trie = TopicTrie::new();
        let engine = Arc::new(Self::create_engine());
        let minijinja_env = Arc::new(Self::create_env());

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
                .new_processor(uuid, engine.clone(), minijinja_env.clone())
                .await;

            match proc {
                Ok(p) => {
                    processor_map.insert(processor.uuid.clone(), p);
                }
                Err(e) => {
                    warn!("failed to create processor {}: {}", processor.uuid, e);
                }
            }
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
            minijinja_env,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<OperatorCommand> {
        self.command_tx.clone()
    }

    pub fn run(&mut self, sparkplug_helper: Option<SparkPlugBApplicationHelper>) {
        let mut command_rx = self.command_rx.take().unwrap();
        let matcher_sender = self.matcher_sender.clone();
        let mut trie = self.trie.take().unwrap();
        let mut cache: HashMap<String, Vec<Chain>> = HashMap::new();
        let chains = self.chains.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(cmd) = command_rx.recv() => {
                        if let OperatorCommand::Publish{client_id, retain, qos, topic, payload, user_properties, options} = cmd {
                            if let Some(ref sparkplug_helper) = sparkplug_helper {
                                if sparkplug_helper.is_sparkplug_b_topic(&topic) {
                                    sparkplug_helper.publish(
                                        client_id.clone(),
                                        retain, qos,
                                        topic.clone(),
                                        payload.clone(),
                                    ).await;
                                }
                            }

                            let chains = Self::find_chain(&mut cache, &mut trie, &chains, &topic, &client_id);
                            if let Some(chains) = chains {
                                let msg = Message::new(
                                    client_id,
                                    topic,
                                    qos,
                                    retain,
                                    payload,
                                    user_properties,
                                ).with_options(options);

                                tokio::spawn( Self::chains_process(chains, msg, matcher_sender.clone()));
                            } else {
                                matcher_sender.send(OperatorCommand::Publish {
                                    client_id,
                                    retain,
                                    qos,
                                    topic,
                                    payload,
                                    user_properties,
                                    options,
                                }).await.ok();
                            }
                        } else if let OperatorCommand::SparkPlugBPublish { client_id, topic, payload, retain, qos } = cmd {
                            let chains = Self::find_chain(&mut cache, &mut trie, &chains, &topic, &client_id);
                            if let Some(chains) = chains {
                                let msg = Message::new(
                                    client_id,
                                    topic,
                                    qos,
                                    retain,
                                    payload,
                                    vec![],
                                );

                                tokio::spawn( Self::chains_process(chains, msg, matcher_sender.clone()));
                            } else {
                                matcher_sender.send(OperatorCommand::Publish {
                                    client_id,
                                    retain,
                                    qos,
                                    topic,
                                    payload,
                                    user_properties: vec![],
                                    options: PublishOptions::default(),
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

    fn find_chain<'a>(
        cache: &'a mut HashMap<String, Vec<Chain>>,
        trie: &'a mut TopicTrie<Chain>,
        chains: &HashMap<String, ProcessorChain>,
        topic: &str,
        client_id: &str,
    ) -> Option<Vec<ProcessorChain>> {
        if !cache.contains_key(topic) {
            let chain = trie
                .find_matches(topic)
                .into_iter()
                .map(|c| c.clone())
                .collect();
            cache.insert(topic.to_string(), chain);
        }

        let find_chains = cache.get(topic).unwrap();
        let chains_name = find_chains
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

    #[allow(dead_code)]
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

        let mut chains_iter = chains.into_iter().peekable();
        while let Some(chain) = chains_iter.next() {
            let processor = |mut msg: Message| {
                set.spawn(async move {
                    for processor in chain.processors {
                        trace!(
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
                })
            };

            if chains_iter.peek().is_none() {
                processor(message);
                break;
            } else {
                processor(message.clone());
            }
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
                            user_properties: msg.user_properties,
                            options: msg.options,
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
