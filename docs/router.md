# Router Documentation

## Overview

The AxonMQ Router is a powerful component that inspects incoming MQTT messages and directs them to one or more **Processor Chains** based on matching rules. This allows you to create custom data pipelines for logging, transformation, alerting, or bridging data to other systems.

The core logic is as follows:
1. A message is published to a topic.
2. The router matches the message's topic (and optionally, the client ID) against all defined router rules.
3. For every matching rule, the message is sent to the corresponding processor chain(s) specified in that rule.
4. **Importantly**, the router acts as a "tap" or a "fork" in the data path. Routing a message to a processor chain does **not** stop it from being delivered to subscribed clients. This is the default behavior unless a processor in the chain explicitly drops the message.

## Configuration

Router rules are defined in your `config.toml` file as an array of `[[router]]` tables. Each table represents a single rule.

### Rule Parameters

A rule consists of the following key-value pairs:

- `topic` (String, Required): An MQTT topic filter that the incoming message's topic is matched against. Standard MQTT wildcards (`+` for single-level and `#` for multi-level) are supported.
- `client_id` (String, Optional): If specified, this rule will only apply to messages published by a client with this exact client ID. If omitted, the rule applies to messages from any client.
- `chain` (Array of Strings, Required): A list of one or more processor chain names. When the rule matches, the message will be sent to all chains listed in this array. The chain names must correspond to chains defined in the `[[chain]]` section of the configuration.

### Configuration Examples

Here are a few examples of how to define router rules in `config.toml`.

**Example 1: Simple Topic Match**

This rule sends all messages published to any sub-topic of `sensors/` (e.g., `sensors/t-01/temperature`) to the "logger" processor chain.

```toml
[[router]]
topic = "sensors/+/temperature"
chain = ["logger"]
```

**Example 2: Topic and Client ID Match**

This rule is more specific. It only matches messages on topics like `sensors/h-01/humidity` if they are published by the client with the ID `sensor_hub_001`.

```toml
[[router]]
topic = "sensors/+/humidity"
client_id = "sensor_hub_001"
chain = ["logger"]
```

**Example 3: Multiple Chains**

This rule sends any message published to `alerts/critical/#` to two different processor chains: one for logging and another for sending notifications.

```toml
[[router]]
topic = "alerts/critical/#"
chain = ["logger", "notification_chain"]
```

### Future Configuration Methods

While the current version requires defining router rules in the `config.toml` file, future releases of AxonMQ will provide more dynamic configuration options, including:

- **HTTP API:** For programmatic creation and management of router rules.
- **Dashboard UI:** A graphical interface for viewing, adding, and editing rules in real-time.

## Topic Matching

The router uses a high-performance trie data structure for efficient topic matching, which fully supports MQTT wildcards:

- `+` (Single-Level Wildcard): Matches any single level in a topic.
  - Example: `a/+/c` matches `a/b/c` but not `a/b/x/c`.
- `#` (Multi-Level Wildcard): Matches any number of levels at the end of a topic. It must be the last character in the filter.
  - Example: `a/b/#` matches `a/b/c`, `a/b/c/d`, and `a/b`.

> **Note:** If a message's topic matches multiple router rules, the message will be sent to the processor chains from **all** matching rules.