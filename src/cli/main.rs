use anyhow::Result;
use clap::Parser;
use reqwest::Client;

mod client;
mod commands;
mod spb;

use commands::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();

    let result = match cli.command {
        Commands::Spb(spb) => spb::handle_spb_command(spb, &cli.host, &client).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}