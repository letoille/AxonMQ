use anyhow::Result;
use clap::Parser;
use coarsetime;
use tokio::runtime::Builder;
use tracing::Level;
use tracing::{info, warn};
use tracing_appender;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::{Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod mqtt;
mod operator;
mod processor;
mod service;
mod utils;

use crate::mqtt::{listener, server};

static CONFIG: std::sync::OnceLock<config::Config> = std::sync::OnceLock::new();

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cmd {
    #[arg(short, long, value_name = "config directory", default_value = "./")]
    config_dir: String,
}

pub fn get_default_log_dir() -> &'static str {
    if cfg!(windows) {
        format!(r"{}\\AxonMQ\\logs\\", std::env::var("ProgramData").unwrap()).leak()
    } else if cfg!(target_os = "macos") {
        "logs"
    } else if cfg!(target_os = "linux") {
        "/var/log/axonmq/"
    } else {
        "logs"
    }
}

fn main() -> Result<()> {
    let cli = Cmd::parse();

    let config = config::Config::from_file(&cli.config_dir)?;
    CONFIG.set(config).unwrap();

    let filter = Targets::new()
        .with_target("axonmq", Level::INFO)
        .with_target("axonmq::service", Level::INFO)
        .with_target("axonmq::service::sparkplug_b", Level::INFO)
        .with_target("axonmq::mqtt", Level::INFO)
        .with_target("axonmq::operator::matcher", Level::INFO)
        .with_target("axonmq::processor::wasm", Level::INFO)
        .with_target("axonmq::processor::processors::logger", Level::INFO);

    let file_appender = tracing_appender::rolling::daily(get_default_log_dir(), "axonmq.log");
    let (nb, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(nb)
        .with_filter(filter.clone());

    let stdout_layer = fmt::layer()
        .with_ansi(true)
        .with_writer(std::io::stdout)
        .with_filter(filter.clone());

    Registry::default()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    let config = CONFIG.get().unwrap();
    info!(
        "Hello, AxonMQ v{}: {}!",
        env!("CARGO_PKG_VERSION"),
        config.node.id
    );

    let runtime = if let Some(core_threads) = config.common.core_threads {
        Builder::new_multi_thread()
            .worker_threads(core_threads)
            .enable_all()
            .build()?
    } else {
        Builder::new_multi_thread().enable_all().build()?
    };

    runtime.block_on(async {
        coarsetime::Updater::new(100).start().unwrap();

        let mut spb_service = if CONFIG.get().unwrap().service.sparkplug_b.enable {
            Some(service::sparkplug_b::SparkPlugBApplication::new())
        } else {
            None
        };
        let spb_helper = spb_service.as_ref().map(|s| s.helper());
        let spb_in_helper = spb_service.as_mut().map(|s| s.in_helper().clone());

        let mut operator = operator::Operator::new().await;
        let operator_helper = operator.helper();
        operator.run(spb_helper.clone());

        if let Some(spb_service) = spb_service.as_mut() {
            spb_service.run(operator_helper.clone()).await;
        }

        let mut broker = server::Broker::new().await;
        let broker_helper = broker.get_helper();
        broker.run(operator_helper.clone()).await;

        let tcp_listener_config = &config.mqtt.listener.tcp;
        listener::spawn_tcp_listener(
            tcp_listener_config.host.clone(),
            tcp_listener_config.port,
            broker_helper.clone(),
            operator_helper.clone(),
        );

        let tls_listener_config = &config.mqtt.listener.tcp_tls;
        listener::spawn_tls_listener(
            tls_listener_config.host.clone(),
            tls_listener_config.port,
            tls_listener_config.cert_path.clone(),
            tls_listener_config.key_path.clone(),
            broker_helper.clone(),
            operator_helper.clone(),
        );

        let ws_listener_config = &config.mqtt.listener.ws;
        listener::spawn_ws_listener(
            ws_listener_config.host.clone(),
            ws_listener_config.port,
            ws_listener_config.path.clone(),
            broker_helper.clone(),
            operator_helper.clone(),
        );

        let wss_listener_config = &config.mqtt.listener.wss;
        listener::spawn_wss_listener(
            wss_listener_config.host.clone(),
            wss_listener_config.port,
            wss_listener_config.path.clone(),
            wss_listener_config.cert_path.clone(),
            wss_listener_config.key_path.clone(),
            broker_helper.clone(),
            operator_helper.clone(),
        );

        if let Ok(restful) =
            service::restful::RESTful::new(&config.service.restful.ip, config.service.restful.port)
        {
            info!("AxonMQ started. Press Ctrl+C to exit.");
            restful.run(spb_in_helper).await;
        } else {
            warn!("Failed to start RESTful service.");
            return Err(anyhow::anyhow!("Failed to start RESTful service."));
        }

        tokio::signal::ctrl_c().await?;
        Ok(())
    })
}
