# Filter Processor Guide

## Overview

The `filter` processor is a powerful tool for controlling the flow of messages within a processor chain. Its sole purpose is to evaluate a condition based on the content of a message and decide whether to **keep** it for further processing or **drop** it from the chain.

## Use Cases

- **Filtering by Value**: Drop messages if a sensor reading is outside an acceptable range.
- **Filtering by Attribute**: Only process messages from a specific client ID, or those published with a certain QoS level.
- **Filtering by Structure**: Drop messages that are missing required fields in their JSON payload.

## Core Logic

The processor's logic is straightforward:

1.  It evaluates the `condition` template against the incoming message.
2.  If the rendered result of the template is **truthy**, the message is **kept** and passed to the next processor in the chain (`Ok(Some(message))`).
3.  If the rendered result is **falsy**, the message is **dropped** (`Ok(None)`).

In `minijinja`, values like `false`, `0`, an empty string `""`, or an empty list `[]` are considered **falsy**. All other values are considered **truthy**.

## Configuration Parameters

The `filter` processor is configured in your `config.toml` file.

```toml
[[processor]]
uuid = "your-filter-processor-uuid"
config = { type = "filter", ... }
```

The `config` table has the following parameters:

| Parameter | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `type` | String | Yes | Must be `"filter"`. | |
| `condition` | String | Yes | A `minijinja` template string that should evaluate to a truthy or falsy value. | |
| `on_error_pass` | Boolean | No | `true` | Defines the behavior when the `condition` template fails to render (e.g., due to a syntax error). If `true`, the message passes through. If `false`, it is dropped. |

## Writing Conditions

The `condition` is a `minijinja` template. You have access to all the standard variables, functions, and filters.

For a complete guide, please see the **[Central Templating Guide](../templating-guide.md)**.

### Best Practice: Handling Missing Fields

When writing a condition, it is crucial to consider what happens if a field you are checking does not exist in the payload. A simple check like `{{ payload.temperature > 40 }}` will evaluate to `false` if `payload.temperature` does not exist, causing the message to be silently dropped.

The recommended way to handle this is to use the `is defined` test in your condition.

**Correct, Robust Condition:**

This condition ensures that you only keep messages that **both contain the `temperature` field AND have a value greater than 40**.

```jinja
{{ payload.temperature is defined and payload.temperature > 40 }}
```

## Full Example

This configuration sets up a filter that only allows high-temperature alerts to be republished.

```toml
[[router]]
topic = "telemetry/thermostat"
chain = ["high_temp_alert_chain"]

[[chain]]
name = "high_temp_alert_chain"
processors = [
    "44444444-4444-4444-4444-444444444444", # UUID for the filter
    "55555555-5555-5555-5555-555555555555"  # UUID for the republisher
]
delivery = false

[[processor]]
uuid = "44444444-4444-4444-4444-444444444444"
config = { \
    type = "filter", \
    # Only keep messages where temperature is defined and greater than 40.
    condition = "{{ payload.temperature is defined and payload.temperature > 40 }}", \
    on_error_pass = true \
}

[[processor]]
uuid = "55555555-5555-5555-5555-555555555555"
config = { type = "republish", topic = "alerts/high_temp" }
```
