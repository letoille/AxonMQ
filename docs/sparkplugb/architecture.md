# Sparkplug Service Architecture

## Overview

The `SparkplugService` is a core component of AxonMQ, acting as a stateful, first-class Sparkplug Host Application. It runs concurrently with the MQTT broker and other services. Its primary responsibilities are:

1.  Maintaining a real-time state model of the entire Sparkplug network topology (Groups, Nodes, Devices, Metrics).
2.  Decoding Sparkplug B payloads and handling the logic for all message types (`BIRTH`, `DEATH`, `DATA`, `COMMAND`).
3.  Providing a thread-safe query interface for other parts of the system (like the RESTful API) to access this state.
4.  Enforcing fault-tolerance rules, such as requesting `Rebirth` when state inconsistencies are detected.

## Core Architecture: The Actor Model

To ensure high performance and avoid the complexities of fine-grained locking, the `SparkplugService` is implemented using the **Actor Model**.

1.  **A Standalone Actor**: The `SparkplugService` runs as a single, independent `async` task within the Tokio runtime.
2.  **Exclusive State Ownership**: This single task **exclusively owns** the state store, which is a standard `std::collections::HashMap`. There are no `Mutex` locks or concurrent maps like `DashMap`.
3.  **Message Passing**: All interactions with the service's state, whether they are updates from new MQTT messages or queries from the API, are handled via asynchronous message passing over `tokio::mpsc` channels.
4.  **Sequential Processing**: The actor's main loop processes one message from its channel at a time. Since all modifications to the `HashMap` happen sequentially within this single task, there are no data races, and no locks are needed. This design is highly efficient, robust, and easy to reason about.

## State Storage Model

The state of the Sparkplug B network is modeled using nested `HashMap`s to precisely mirror the official topology.

```rust
// Simplified structure

// The top-level state store owned by the SparkplugService actor
struct SparkplugStateStore {
    // Key: "{group_id}/{node_id}"
    nodes: HashMap<String, NodeState>,
}

pub struct NodeState {
    pub metrics: HashMap<String, Metric>,
    // Key: "{device_id}"
    pub devices: HashMap<String, DeviceState>,
    // ... other node metadata
}

pub struct DeviceState {
    pub metrics: HashMap<String, Metric>,
    // ... other device metadata
}

pub struct Metric {
    pub value: serde_json::Value,
    pub timestamp: Instant,
    // ... other metric metadata
}
```

## Message Processing Flow

1.  **Interception**: The `Operator` component intercepts all messages published to the `spBv1.0/#` topic space. Instead of immediately sending them to the `Router` for processor chain execution, it first forwards them to the `SparkplugService`'s message channel.
2.  **Decoding**: The `SparkplugService` actor receives the message, decodes its Protobuf payload, and determines the message type (`NBIRTH`, `DDATA`, etc.).
3.  **State Update**: Based on the message type, the actor performs the corresponding action on its internal `HashMap` state store:
    *   **`BIRTH`**: Creates or updates entries for Nodes and Devices, populating their entire metric sets.
    *   **`DEATH`**: Marks the corresponding Node or Device as offline and all its metrics as stale.
    *   **`DATA`**: Traverses the state tree to find the specific metrics and updates their values and timestamps.
4.  **Forwarding (Future)**: After processing, the message (potentially enriched with metadata) will be sent back to the `Router` to be processed by user-defined processor chains.

## Fault Tolerance: Rebirth Mechanism

The service implements fault-tolerance logic as recommended by the Sparkplug B specification. When a state inconsistency is detected, the service can automatically send an `NCMD` message to request a `Rebirth` from the problematic Edge Node.

This behavior is controlled by the `[sparkplug.rebirth_on_error]` section in `config.toml`:

| Parameter | Default | Description |
| :--- | :--- | :--- |
| `on_sequence_mismatch` | `"ignore"` | Controls behavior when a message `seq` number gap is detected. Defaults to `ignore` to prevent "rebirth storms" on unstable networks. |
| `on_malformed_payload` | `"request"` | Controls behavior when a payload cannot be decoded. Defaults to `request` as this is a critical error. |

The service will **always** request a `Rebirth` for the following critical scenarios, as this indicates a definite state inconsistency:
- Receiving a message for a Node or Device that has not sent a `BIRTH` certificate.
- Receiving a `DATA` message with a metric or alias that was not defined in the corresponding `BIRTH` certificate.

## Internal Query Interface

To allow other concurrent services (like the RESTful API) to safely query the state without direct access or locks, a `request-response` channel pattern is used:

1.  The API handler (the "client") creates a `tokio::oneshot` channel.
2.  It sends a query message (e.g., `GetNode { node_id: "...", responder: oneshot_tx }`) to the `SparkplugService`'s main channel. The message contains the `sender` half of the `oneshot` channel.
3.  The API handler then `.await`s the `receiver` half of the `oneshot` channel.
4.  The `SparkplugService` actor processes the query message, retrieves the requested data from its `HashMap`, and sends the result back via the provided `responder` channel.
5.  The API handler's `await` completes, and it receives the data, which it can then serialize as a JSON HTTP response.

This ensures the entire system remains non-blocking, thread-safe, and highly performant.
