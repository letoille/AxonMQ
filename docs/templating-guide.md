# AxonMQ Templating Guide (minijinja)

Several processors within AxonMQ, such as `webhook`, `json-transform`, and `republish`, use the `minijinja` templating engine to provide powerful and dynamic functionality. This guide serves as a central reference for how to use templates in AxonMQ.

## Basic Syntax

The templating engine has two primary syntaxes:

- `{{ ... }}`: **Expressions**. Used to print the value of a variable or the result of an expression. For example: `{{ client_id }}`.
- `{% ... %}`: **Statements**. Used for logic and control flow, such as conditionals (`if`) and loops (`for`). These do not directly output content.

## Context Variables

When a template is rendered for a message, you have access to the following variables in the context:

| Variable | Type | Description |
| :--- | :--- | :--- |
| `{{ client_id }}` | String | The ID of the client that published the message. |
| `{{ topic }}` | String | The topic the message was published to. |
| `{{ qos }}` | Integer | The Quality of Service level of the message (0, 1, or 2). |
| `{{ retain }}` | Boolean | The retain flag of the message (`true` or `false`). |
| `{{ payload }}` | JSON Object | The message payload, automatically parsed as a JSON object. You can access its fields like `{{ payload.temperature }}`. If parsing fails, it will be `null`. |
| `{{ raw_payload }}` | String | The raw message payload, interpreted as a UTF-8 string. |
| `{{ metadata }}` | Map | The message metadata map. You can access values with `{{ metadata.my_key }}`. |

## Custom Functions & Filters

AxonMQ adds the following custom functions and filters to the environment for your convenience:

- **`now()` function**: Returns the current Unix timestamp (milliseconds since epoch) as a number.
  - **Usage**: `{{ now() }}`

- **`date` filter**: Formats a Unix timestamp (milliseconds since epoch) into a **UTC** date string.
  - **Usage**: `{{ my_timestamp | date(format) }}`
  - **Arguments**:
    - `format` (optional, string): An `strftime` format string. Defaults to `%Y-%m-%d %H:%M:%S UTC`.

## Common Built-in Filters

Here are some of the most useful built-in filters that come with `minijinja`.

#### String Manipulation

| Filter | Description | Example |
| :--- | :--- | :--- |
| `upper` | Converts a string to uppercase. | `{{ "hello" | upper }}` → `HELLO` |
| `lower` | Converts a string to lowercase. | `{{ "WORLD" | lower }}` → `world` |
| `replace` | Replaces a substring. | `{{ "a-b-c" | replace("-", "_") }}` → `a_b_c` |
| `truncate` | Truncates a string to a given length. | `{{ client_id | truncate(8) }}` → `sensor-a...` |

#### Logic & Safety

| Filter | Description | Example |
| :--- | :--- | :--- |
| `default` | Provides a default value if a variable is undefined or null. | `{{ username | default(value="guest") }}` |
| `tojson` | Serializes a variable into a JSON formatted string. | `{{ payload | tojson }}` → `"{\"temp\":25}"` |

#### List/Array Manipulation

| Filter | Description | Example |
| :--- | :--- | :--- |
| `length` | Returns the length of a string or a list. | `{{ [1, 2, 3] | length }}` → `3` |
| `join` | Joins list elements with a separator. | `{{ ["a", "b"] | join(sep=", ") }}` → `a, b` |
| `first` | Gets the first element of a list. | `{{ ["a", "b"] | first }}` → `a` |

For a complete list of all available filters, please refer to the [official minijinja documentation](https://docs.rs/minijinja/latest/minijinja/filters/).

## Advanced Example: Batch Unpacking

This example shows how to use a `for` loop to process a message that contains an array of metrics.

**Incoming Payload:**
```json
{
  "device_id": "sensor-01",
  "metrics": [
    { "name": "temperature", "value": 25.7 },
    { "name": "humidity", "value": 61.2 }
  ]
}
```

**Template:**
```jinja
[
  {% for metric in payload.metrics %}
    {
      "measurement": "{{ metric.name }}",
      "tags": { "device_id": "{{ payload.device_id }}" },
      "fields": { "value": {{ metric.value }} },
      "timestamp": {{ now() }}
    }{% if not loop.last %},{% endif %}
  {% endfor %}
]
```

**Resulting Payload:**
```json
[
  {
    "measurement": "temperature",
    "tags": { "device_id": "sensor-01" },
    "fields": { "value": 25.7 },
    "timestamp": 1678886400
  },
  {
    "measurement": "humidity",
    "tags": { "device_id": "sensor-01" },
    "fields": { "value": 61.2 },
    "timestamp": 1678886400
  }
]
```