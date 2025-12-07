use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The base URL of the AxonMQ broker
    #[arg(long, global = true, default_value = "http://127.0.0.1:1107")]
    pub host: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Access Sparkplug B data
    Spb(Spb),
}

#[derive(Parser)]
pub struct Spb {
    #[command(subcommand)]
    pub command: SpbCommands,
}

#[derive(Subcommand)]
pub enum SpbCommands {
    /// Get Sparkplug B resources
    Get(Get),
    /// Set Sparkplug B resources
    Set(Set),
}

#[derive(Parser)]
pub struct Get {
    #[command(subcommand)]
    pub resource: GetResource,
}

#[derive(Subcommand)]
pub enum GetResource {
    /// Get all groups
    Groups,
    /// Get a specific group
    Group {
        #[arg(required = true)]
        group_id: String,
    },
    /// Get all nodes in a group
    Nodes {
        #[arg(required = true)]
        group_id: String,
    },
    /// Get a specific node in a group
    Node {
        #[arg(required = true)]
        group_id: String,
        #[arg(required = true)]
        node_id: String,
    },
    /// Get all devices in a node
    Devices {
        #[arg(required = true)]
        group_id: String,
        #[arg(required = true)]
        node_id: String,
    },
    /// Get a specific device in a node
    Device {
        #[arg(required = true)]
        group_id: String,
        #[arg(required = true)]
        node_id: String,
        #[arg(required = true)]
        device_id: String,
    },
}

#[derive(Parser)]
pub struct Set {
    #[command(subcommand)]
    pub resource: SetResource,
}

#[derive(Subcommand)]
pub enum SetResource {
    /// Set metrics on a node
    Node {
        #[arg(required = true)]
        group_id: String,
        #[arg(required = true)]
        node_id: String,
        /// Metrics to set, in key=value format
        #[arg(required = true)]
        metrics: Vec<String>,
    },
    /// Set metrics on a device
    Device {
        #[arg(required = true)]
        group_id: String,
        #[arg(required = true)]
        node_id: String,
        #[arg(required = true)]
        device_id: String,
        /// Metrics to set, in key=value format
        #[arg(required = true)]
        metrics: Vec<String>,
    },
}
