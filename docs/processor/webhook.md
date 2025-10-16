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

The `body_template` parameter gives you full control over the format of the outgoing HTTP request body. It uses the `minijinja` templating engine.

Within the template, you have access to the following variables from the MQTT message:

| Variable | Type | Description |
| :--- | :--- | :--- |
| `{{ client_id }}` | String | The ID of the client that published the message. |
| `{{ topic }}` | String | The topic the message was published to. |
| `{{ qos }}` | Integer | The Quality of Service level of the message (0, 1, or 2). |
| `{{ retain }}` | Boolean | The retain flag of the message (`true` or `false`). |
| `{{ payload }}` | JSON Object | The message payload, automatically parsed as a JSON object. You can access its fields like `{{ payload.temperature }}`. If parsing fails, it will be `null`. |
| `{{ raw_payload }}` | String | The raw message payload, interpreted as a UTF-8 string. |

### Template Example

**If `body_template` is:**
```jinja
{
    "device": "{{ client_id }}",
    "metric": {
        "temp": {{ payload.temp }},
        "unit": "celsius"
    },
    "retained_msg": {{ retain }}
}
```

**And the incoming MQTT message payload is:**
```json
{"temp": 25.7, "humidity": 60}
```

**The resulting HTTP request body will be:**
```json
{
    "device": "sensor-01",
    "metric": {
        "temp": 25.7,
        "unit": "celsius"
    },
    "retained_msg": false
}
```

### Using Filters

Filters are functions that can be applied to variables to modify their values before rendering. They use the pipe `|` symbol. You can chain multiple filters together.

**Syntax:** `{{ variable | filter_name(argument) }}`

Here are some commonly used built-in filters:

| Filter | Description | Example |
| :--- | :--- | :--- |
| `upper` | Converts a string to uppercase. | `{{ "hello" \| upper }}` → `HELLO` |
| `lower` | Converts a string to lowercase. | `{{ "WORLD" \| lower }}` → `world` |
| `replace` | Replaces a substring. | `{{ "a-b-c" \| replace("-", "_") }}` → `a_b_c` |
| `truncate` | Truncates a string to a given length. | `{{ client_id \| truncate(8) }}` → `sensor-a...` |
| `default` | Provides a default value if the variable is undefined or null. | `{{ username \| default(value="guest") }}` |
| `length` | Returns the length of a string or a list. | `{{ [1, 2, 3] \| length }}` → `3` |
| `round` | Rounds a number. Can take `method="ceil"` or `method="floor"`. | `{{ 42.7 \| round }}` → `43` |
| `tojson` | Serializes a variable into a JSON string. | `{{ payload \| tojson }}` → `"{\"temp\":25}"` |

**Combined Example:**

```jinja
{
  "device_id_upper": "{{ client_id | upper }}",
  "short_topic": "{{ topic | truncate(20) }}",
  "user": "{{ username | default(value='system') }}",
  "payload_as_string": "{{ payload | tojson }}"
}
```

For a complete list of all available filters, please refer to the [official minijinja documentation](https://docs.rs/minijinja/latest/minijinja/filters/).

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