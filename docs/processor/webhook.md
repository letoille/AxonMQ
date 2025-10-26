# Webhook Processor Guide

## Overview

The Webhook processor is a powerful tool for data integration. It forwards MQTT messages from AxonMQ to an external HTTP endpoint, allowing you to trigger backend services, notify applications, or archive data in real-time.

For each message it receives, it asynchronously sends an HTTP request to a configured URL. It includes robust features like concurrency control, templating, and custom headers to handle real-world use cases.

## Use Cases

- **Real-time Notifications**: Notify a backend service about new device events.
- **Serverless Triggers**: Trigger functions on platforms like AWS Lambda, Google Cloud Functions, or Azure Functions via their HTTP endpoints.
- **Data Archiving**: Send message data to a data-collection API that stores it in a database or data lake.
- **System Integration**: Connect AxonMQ to other internal enterprise systems that expose REST APIs.

## Configuration Parameters

The `webhook` processor is configured in your `config.toml` file under a `[[processor]]` table.

```toml
[[processor]]
uuid = "your-webhook-processor-uuid"
config = { type = "webhook", ... }
```

The `config` table has the following parameters:

| Parameter | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `type` | String | Yes | `"webhook"` | Specifies the processor type. |
| `url` | String | Yes | | The target HTTP(S) endpoint URL. |
| `method` | String | No | `"POST"` | The HTTP method to use (e.g., `POST`, `PUT`). |
| `headers` | Table | No | (empty) | A key-value map of custom HTTP headers to send with the request. |
| `body_template` | String | No | (raw payload) | A [minijinja](https://docs.rs/minijinja/latest/minijinja/) template for the request body. If not specified, the raw MQTT message payload is used as the body. |
| `timeout_secs` | Integer | No | `10` | The HTTP request timeout in seconds. |
| `max_concurrency` | Integer | No | `100` | The maximum number of concurrent in-flight HTTP requests allowed for this processor instance. |

## Body Templating

The `body_template` parameter gives you full control over the format of the outgoing HTTP request body by using the `minijinja` templating engine.

This allows you to use variables from the MQTT message, and leverage powerful filters and logic to create any structure you need. For a complete guide on syntax, available variables, custom functions like `now()`, and filters, please see the **[Central Templating Guide](../templating-guide.md)**.

## Full Example

This configuration sets up a webhook that listens for messages on `data/ingest/#` and forwards a custom JSON payload to a test endpoint.

```toml
# 1. Router: Forwards messages from "data/ingest/#" to the webhook chain.
[[router]]
topic = "data/ingest/#"
chain = ["webhook_chain"]

# 2. Chain: Contains the webhook processor.
[[chain]]
name = "webhook_chain"
processors = ["b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12"]
delivery = true

# 3. Processor: Configures the webhook endpoint and template.
[[processor]]
uuid = "b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12"
config = { \
    type = "webhook", \
    url = "https://webhook.site/YOUR_UNIQUE_ID_HERE", \
    method = "POST", \
    max_concurrency = 50, \
    headers = { "Content-Type" = "application/json", "X-Source" = "AxonMQ" }, \
    body_template = '''
{
    "topic": "{{ topic }}",
    "payload": {{ payload }}
}
''' \
}
```