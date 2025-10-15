# Developing WebAssembly (WASM) Processors

This document describes how to develop a processor for AxonMQ using WebAssembly (WASM), based on the WIT (Wasm Interface Type) definition.

## Overview

WASM processors offer a powerful way to extend AxonMQ with enhanced security, dynamic loading, and language independence.

- **Security**: Processors run in a secure sandbox, isolated from the main broker process. They can only interact with the host through a well-defined interface.
- **Dynamic Loading**: WASM modules can be loaded and updated at runtime without recompiling or restarting the broker.
- **Language-Independent**: You can write processors in any language that compiles to the `wasm32-wasi` target and supports WIT bindings, such as Rust, C/C++, Go, and more.

The interaction between AxonMQ (the host) and the WASM module (the guest) is strictly defined by the WIT contract.

## The WIT Contract (`wit/processor.wit`)

The file `wit/processor.wit` defines the Application Binary Interface (ABI). Any WASM module you create **must** implement the `handler` interface and can import the `logging` interface.

```wit
package axonmq:processor;

interface handler {
    // ... (records and enums for message structure)

    // Main function to process a message
    on-message: func(message: message) -> result<message-result, string>;

    // Functions to provide metadata about the processor
    name: func() -> string;
    version: func() -> string;
    description: func() -> string;

    // Functions called by the host to provide context
    set-instance-id: func(id: string);
    set-config: func(config: string);
}

interface logging {
    // Function to log messages from within the WASM module
    log: func(level: log-level, target: string, message: string);
}

world axonmq-processor {
    export handler;
    import logging;
}
```

### Exported `handler` Interface

Your WASM module must export and implement these functions:

- `on-message(message: message) -> result<message-result, string>`: The core function, called for each message.
  - **Returns**:
    - `Ok(forward(message))`: To pass the original or a modified message onward.
    - `Ok(drop)`: To stop the message from proceeding.
    - `Err(string)`: To signal a processing error.
- `name()`, `version()`, `description()`: Return metadata about your processor.
- `set-instance-id(id: string)`: The host calls this to provide the processor's UUID.
- `set-config(config: string)`: The host calls this to pass the JSON configuration string from `config.toml`.

### Imported `logging` Interface

Your WASM module can import and use the `log` function to send log messages back to the AxonMQ host, which will write them to its standard log output.

## Step-by-Step Implementation Guide (using Rust)

This guide explains how to create a WASM processor in Rust using `wit-bindgen`.

### Step 1: Project Setup

1.  Create a new Rust library project:
    ```bash
    cargo new --lib my-wasm-processor
    cd my-wasm-processor
    ```

2.  Add `wit-bindgen` as a dependency in `Cargo.toml`:
    ```toml
    [dependencies]
    wit-bindgen = { version = "0.24.0", features = ["macros"] }
    ```

3.  Configure `Cargo.toml` to produce a `wasm32-wasi` binary:
    ```toml
    [lib]
    crate-type = ["cdylib"]
    ```

4.  Copy the `wit` directory from the AxonMQ project into your new project's root.

### Step 2: Implement the WIT Interface

In `src/lib.rs`, use `wit_bindgen::generate!` to import the interfaces and define your component.

```rust
// Use wit-bindgen to generate the traits and types from the .wit file
wit_bindgen::generate!({
    world: "axonmq-processor",
    path: "wit",
    exports: {
        "axonmq:processor/handler": MyProcessor,
    }
});

// Define a struct that will represent our processor
struct MyProcessor;

// Implement the generated `Guest` trait for our struct
impl exports::axonmq::processor::handler::Guest for MyProcessor {
    fn name() -> String { "My Awesome WASM Processor".to_string() }
    fn version() -> String { "0.1.0".to_string() }
    fn description() -> String { "Processes messages in a very awesome way.".to_string() }

    fn set_instance_id(id: String) {
        // You can store this ID if needed, e.g., in a static variable
        // For logging, you can use the imported logging interface
        logging::log(logging::LogLevel::Info, &name(), &format!("Instance ID set: {}", id));
    }

    fn set_config(config: String) {
        // Parse the JSON config string and store it if needed
        logging::log(logging::LogLevel::Info, &name(), &format!("Config received: {}", config));
    }

    fn on_message(mut msg: Message) -> Result<MessageResult, String> {
        logging::log(
            logging::LogLevel::Debug,
            &name(),
            &format!("Processing message on topic: {}", msg.topic),
        );

        // Example: Add a metadata field
        msg.metadata.push((
            "processed-by-wasm".to_string(),
            Value::VString("my-processor-0.1.0".to_string()),
        ));

        // Forward the modified message
        Ok(MessageResult::Forward(msg))
    }
}
```

### Step 3: Compile to WASM

Compile your project to the `wasm32-wasi` target.

```bash
cargo build --target wasm32-wasi --release
```

Your compiled WASM module will be at `target/wasm32-wasi/release/my_wasm_processor.wasm`.

### Step 4: Configure AxonMQ

In your `config.toml`, define a `[[processor]]` of type `wasm` and point it to your compiled module.

- `type`: Must be `"wasm"`.
- `path`: The path to your `.wasm` file.
- `cfg`: A JSON string that will be passed to your processor's `set-config` function.

```toml
# 1. Define the WASM processor instance
[[processor]]
uuid = "YOUR-NEW-WASM-UUID-HERE"
config = { type = "wasm", path = "path/to/my_wasm_processor.wasm", cfg = "{ \"threshold\": 42 }" }

# 2. Use it in a chain
[[chain]]
name = "wasm_chain"
processors = ["YOUR-NEW-WASM-UUID-HERE"]
delivery = true
```