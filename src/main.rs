use anyhow::Result;
use clap::Parser;
use coarsetime;
use tracing::Level;
use tracing::info;
use tracing_appender;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::{Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod mqtt;
mod operator;
mod utils;

use crate::mqtt::server;

static CONFIG: std::sync::OnceLock<config::Config> = std::sync::OnceLock::new();

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cmd {
    #[arg(short, long, value_name = "config.toml", default_value = "config.toml")]
    config: String,
}

fn get_default_log_dir() -> &'static str {
    if cfg!(windows) {
        format!("{}\\AxonMQ\\logs\\", std::env::var("ProgramData").unwrap()).leak()
    } else if cfg!(unix) {
        "/var/log/axonmq/"
    } else {
        "."
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cmd::parse();

    let config = config::Config::from_file(&cli.config)?;
    CONFIG.set(config).unwrap();

    let filter = Targets::new()
        .with_target("axonmq", Level::INFO)
        .with_target("axonmq::operator::router", Level::INFO);

    let file_appender = tracing_appender::rolling::daily(get_default_log_dir(), "axonmq.log");
    let (nb, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(nb)
        .with_filter(filter.clone());

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter.clone());

    Registry::default()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    let config = CONFIG.get().unwrap();
    info!("Hello, AxonMQ: {}!", config.node.id);

    coarsetime::Updater::new(100).start().unwrap();
    server::Broker::new(&config.mqtt.listener.host, config.mqtt.listener.port)
        .await
        .run()
        .await;
    Ok(())
}
