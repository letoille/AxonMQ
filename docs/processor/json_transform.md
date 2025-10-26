# Json-Transform Processor Guide

## Overview

The `json-transform` processor is one of the most powerful and flexible tools in the AxonMQ processing pipeline. Its core function is to take an incoming message with a JSON payload and use a [minijinja](https://docs.rs/minijinja/latest/minijinja/) template to create a new, transformed JSON payload.

This allows you to reshape, enrich, filter, and restructure data on the fly as it passes through the broker.

## Use Cases

- **Data Enrichment**: Add new fields to a JSON payload, such as a server-side timestamp, static metadata, or information from other message properties.
- **Data Restructuring**: Adapt the structure of an incoming JSON payload to match the format required by a downstream system, such as a database or a specific API for a webhook.
- **Data Filtering/Extraction**: Create a new, smaller JSON object that only contains a subset of the fields from the original payload.
- **Batch Processing**: Unpack a JSON array of measurements into a different array structure, preparing it for batch database writes.

## Configuration Parameters

The `json-transform` processor is configured in your `config.toml` file under a `[[processor]]` table.

```toml
[[processor]]
uuid = "your-transform-processor-uuid"
config = { type = "json_transform", ... }
```

The `config` table has the following parameters:

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `type` | String | Yes | Must be `"json_transform"`. |
| `template` | String | Yes | A `minijinja` template string that defines how the new JSON payload should be constructed. |

**Behavior Note:** If the incoming message's payload is not a valid JSON string, this processor will log a warning and pass the message through without any transformation.

## Templating Guide

The `template` parameter uses the `minijinja` templating engine to define how the new JSON payload is constructed.

This allows you to use variables from the MQTT message, and leverage powerful filters and logic to create any structure you need. For a complete guide on syntax, available variables, custom functions like `now()`, and filters, please see the **[Central Templating Guide](../templating-guide.md)**.
