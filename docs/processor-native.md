# Developing Native Rust Processors

This document describes how to develop a native processor for AxonMQ directly in Rust.

## Overview

Native processors are Rust components compiled directly into the AxonMQ binary. They offer the best possible performance and have full access to the broker's internal APIs. This approach is ideal for performance-critical or tightly integrated functionalities.

Every native processor must implement the `Processor` trait.

## The `Processor` Trait

The core of any processor is the `Processor` trait defined in `src/processor/mod.rs`. It defines the essential behavior of a processor.

```rust
#[async_trait]
pub trait Processor: Send + Sync + DynClone {
    fn id(&self) -> Uuid;
    fn as_any(&self) -> &dyn Any;

    async fn on_message(
        &self,
        message: message::Message,
    ) -> Result<Option<message::Message>, error::ProcessorError>;
}
```

### Trait Methods

- `id(&self) -> Uuid`: Returns the unique identifier (UUID) of this processor instance. This ID is used in `config.toml` to add the processor to a chain.
- `as_any(&self) -> &dyn Any`: Used for downcasting, typically for debugging or special handling.
- `on_message(...)`: This is the main execution function. It's called when a message is passed to the processor.
  - **Input**: It receives a `Message` object.
  - **Output**: It returns a `Result<Option<Message>, ProcessorError>`.
    - `Ok(Some(message))`: The message is passed on to the next processor in the chain. You can return the original message or a modified one.
    - `Ok(None)`: The message is "dropped." It will not proceed further down the chain and will not be delivered to the subscriber (if this chain was the final one).
    - `Err(ProcessorError)`: An error occurred. The message processing is halted.

## Configuration

Processors and their sequences (Chains) are defined in `config.toml`.

### 1. Defining a Processor Instance (`[[processor]]`)

Each processor instance is given a unique UUID and configured.

- `uuid` (String, Required): A unique UUID for this processor instance. You can generate one using `uuidgen`.
- `config` (Table, Required): A configuration table. It must have a `type` field that matches the name defined in `src/processor/config.rs`.

**Example for the built-in Logger:**

```toml
[[processor]]
uuid = "10495c56-1922-414e-acfe-0bffafaa5d12"
config = { type = "logger", level = "info" }
```

### 2. Arranging Processors in a Chain (`[[chain]]`)

Chains define the execution order of processors.

- `name` (String, Required): A unique name for the chain. This name is used in `[[router]]` rules.
- `processors` (Array of UUIDs, Required): An ordered list of processor `uuid`s to be executed sequentially.
- `delivery` (Boolean, Required): If `true`, the message that successfully exits the chain will be sent to the subscription matcher for delivery. If `false`, it will not be delivered, even if it wasn't dropped.

**Example:**

```toml
[[chain]]
name = "logger"
processors = ["10495c56-1922-414e-acfe-0bffafaa5d12"]
delivery = true
```

## Step-by-Step Implementation Guide

Let's use the existing `LoggerProcessor` as a guide to create a new native processor.

### Step 1: Create the Processor File

Create a new file for your processor, for example, `src/processor/processors/my_processor.rs`.

### Step 2: Define the Struct and Implement `Processor`

In your new file, define a struct and implement the `Processor` trait for it.

```rust
use async_trait::async_trait;
use uuid::Uuid;
// ... other necessary imports

#[derive(Clone)]
pub struct MyProcessor {
    id: Uuid,
    // ... other fields for state
}

#[async_trait]
impl Processor for MyProcessor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn on_message(&self, message: Message) -> Result<Option<Message>, ProcessorError> {
        // Your logic here. For example, print the payload.
        println!("MyProcessor received payload: {:?}", message.payload);

        // Pass the message to the next processor.
        Ok(Some(message))
    }
}
```

### Step 3: Create a Constructor and Handle Configuration

You need a way to create an instance of your processor and handle its specific configuration from `config.toml`.

1.  **Add your config type to `ProcessorConfig` enum** in `src/processor/config.rs`:

    ```rust
    // In src/processor/config.rs
    #[derive(Debug, Deserialize, Clone)]
    #[serde(tag = "type")]
    pub enum ProcessorConfig {
        #[serde(rename = "logger")]
        Logger { level: String },
        #[serde(rename = "wasm")]
        Wasm { path: String, cfg: String },

        // Add your new processor type here
        #[serde(rename = "my_processor")]
        MyProcessor { my_param: String },

        #[serde(other)]
        Other,
    }
    ```

2.  **Implement a constructor** for your processor struct that takes the config.

    ```rust
    // In src/processor/processors/my_processor.rs
    impl MyProcessor {
        pub fn new_with_id(
            id: Uuid,
            config: ProcessorConfig,
        ) -> Result<Box<dyn Processor>, ProcessorError> {
            if let ProcessorConfig::MyProcessor { my_param } = config {
                // Use my_param to initialize your processor
                println!("Initializing MyProcessor with param: {}", my_param);
                Ok(Box::new(MyProcessor { id }))
            } else {
                Err(ProcessorError::InvalidConfiguration(
                    "Invalid configuration for MyProcessor".to_string(),
                ))
            }
        }
    }
    ```

### Step 4: Register the Processor

In `src/processor/config.rs`, add your new processor to the `new_processor` factory function.

```rust
// In src/processor/config.rs
impl ProcessorConfig {
    pub async fn new_processor(...) -> Result<Box<dyn Processor>, String> {
        match self {
            ProcessorConfig::Logger { .. } => { ... }
            ProcessorConfig::Wasm { .. } => { ... }

            // Add your processor's construction logic
            ProcessorConfig::MyProcessor { .. } => {
                my_processor::MyProcessor::new_with_id(id, self.clone()).map_err(|e| e.to_string())
            }

            ProcessorConfig::Other => Err("Unsupported processor type".to_string()),
        }
    }
}
```

### Step 5: Use in `config.toml`

You can now configure and use your new native processor.

1.  Generate a new UUID for your processor instance.
2.  Add it to the `[[processor]]` section.
3.  Add its UUID to a `[[chain]]`.

```toml
# 1. Define the processor instance
[[processor]]
uuid = "YOUR-NEW-UUID-HERE"
config = { type = "my_processor", my_param = "hello_world" }

# 2. Use it in a chain
[[chain]]
name = "my_chain"
processors = ["YOUR-NEW-UUID-HERE"]
delivery = true
```