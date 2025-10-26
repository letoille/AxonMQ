# Republish Processor Guide

## Overview

The `republish` processor is a versatile tool used to take an incoming message and publish it again within the broker, but with modified properties. Its primary use is to change the message's topic, but it can also override the QoS, retain flag, and even the payload.

When a message is processed by the `republish` processor, it is transformed according to the configuration and then passed back to the broker's core for routing and delivery based on its new properties. This allows for powerful message routing logic and workflow creation.

## Use Cases

- **Topic Aliasing**: Forward messages from an internal, complex topic structure to a simpler, public-facing topic.
- **Dynamic Routing**: Use templates to route messages to topics based on their content or properties (e.g., route messages from different clients to client-specific topics).
- **Event Generation**: Publish a new, static message to a status topic in response to an incoming message.
- **Message Duplication**: In combination with the router, a single incoming message can be sent to multiple chains, each with a `republish` processor, to duplicate the message to several different topics.

## Configuration Parameters

The `republish` processor is configured in your `config.toml` file under a `[[processor]]` table.

```toml
[[processor]]
uuid = "your-republish-processor-uuid"
config = { type = "republish", ... }
```

The `config` table has the following parameters:

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `type` | String | Yes | Must be `"republish"`. |
| `topic` | String | Yes | A template for the new topic to publish the message to. |
| `qos` | Integer | No | If specified, overrides the original message's QoS (0, 1, or 2). |
| `retain` | Boolean | No | If specified, overrides the original message's retain flag. |
| `payload` | String | No | If specified, replaces the original message's payload with this static string content. |

## Templating

The `topic` parameter supports templating, allowing you to create dynamic topics based on the incoming message.

For a complete guide on syntax, available variables (like `{{ client_id }}`), custom functions like `now()`, and filters, please see the **[Central Templating Guide](../templating-guide.md)**.

### Template Example

**If the `topic` parameter in your config is:**
```
topic = "processed/{{ client_id }}/data"
```

**And the incoming message is from `client_id = "sensor-123"` on topic `raw/data`...**

**The resulting message will be republished to the topic:**
```
processed/sensor-123/data
```

## Full Example

This configuration demonstrates a chain where a message's payload is first transformed, and then the new message is republished to a dynamic topic.

```toml
[[router]]
topic = "raw/json/#"
chain = ["transform_and_republish_chain"]

[[chain]]
name = "transform_and_republish_chain"
processors = [
    "f47ac10b-58cc-4372-a567-0e02b2c3d479", # UUID for the transformer
    "98a34e3d-ff2d-4dc9-a52b-34a5c2c3d480"  # UUID for the republisher
]
delivery = true

# 1. The transform processor prepares the new payload
[[processor]]
uuid = "f47ac10b-58cc-4372-a567-0e02b2c3d479"
config = { type = "json_transform", template = "{ \"data\": {{ payload }}, \"processed_at\": {{ now() }} }" }

# 2. The republish processor sends the transformed message to a new, dynamic topic
[[processor]]
uuid = "98a34e3d-ff2d-4dc9-a52b-34a5c2c3d480"
config = { type = "republish", topic = "processed/{{ client_id }}/data" }
```