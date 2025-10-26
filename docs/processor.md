# Processor Documentation

## Overview

The AxonMQ processor module allows for powerful customization of the data processing pipeline by allowing you to inspect, modify, or route messages as they pass through the broker.

## Core Concepts

### Processor Chains

A **Processor Chain** is an ordered sequence of one or more processors that a message passes through. Chains are defined in `config.toml` and are given a unique name. A router rule then links a topic to a specific chain.

### The `delivery` Flag

Each chain has a `delivery` flag. This boolean flag controls what happens to a message after it has been successfully processed by all processors in the chain.

- `delivery = true`: The message that exits the chain (which may have been modified) is sent to the **Subscription Matcher** to be delivered to subscribed clients.
- `delivery = false`: The message is consumed by the chain and will not be delivered to subscribers, even if it was not explicitly dropped by a processor. This is useful for creating processing pipelines that only perform side-effects (like writing to a database) without sending the message onward.

## Development Guides

You can create processors in two ways, each suited for different use cases. Follow the guide that best fits your needs:

- **[Native Rust Processors](./processor-native.md)**: For maximum performance and tight integration with the broker.
- **[WebAssembly (WASM) Processors](./processor-wasm.md)**: For security, dynamic loading, and language independence.

## Built-in Processors

AxonMQ comes with some processors that can be used out-of-the-box.

### Native Processors

| Processor | Description | Source Code |
| :--- | :--- | :--- |
| **Logger** | Logs received messages to the console. The log level (`info`, `debug`, etc.) is configurable. | `src/processor/processors/logger.rs` |
| **Republish** | Forwards a message to a new, potentially dynamic topic. See the **[detailed guide](./processor/republish.md)**. | `src/processor/processors/republish.rs` |
| **Webhook** | Sends message data to an external HTTP endpoint. See the **[detailed guide](./processor/webhook.md)**. | `src/processor/processors/webhook.rs` |
| **Json-Transform** | Transforms a JSON payload using a minijinja template. See the **[detailed guide](./processor/json_transform.md)**. | `src/processor/processors/json_transform.rs` |

### WebAssembly (WASM) Processors

| Processor | Description | Location |
| :--- | :--- | :--- |
| **Example** | A simple example processor that demonstrates the WASM ABI. It adds a metadata field to the message. | `wasm/example.wasm` |

## Future Configuration Methods

While the current version requires defining processors and chains in the `config.toml` file, future releases of AxonMQ will provide more dynamic configuration options, including:

- **HTTP API:** For programmatic creation and management of processors and chains.
- **Dashboard UI:** A graphical interface for viewing, adding, and editing your processing pipelines in real-time.
