# AxonMQ CLI Usage Guide

The `axonmq-cli` is a command-line interface to interact with a running AxonMQ broker. It currently focuses on accessing and manipulating Sparkplug B data via the broker's RESTful API.

## Global Options

- `--host <URL>`: Specifies the base URL of the AxonMQ broker.
  - **Default**: `http://127.0.0.1:1107`

---

## Commands

All Sparkplug B related commands are nested under the `spb` subcommand.

### `spb get`

The `get` command is used to retrieve and display the state of various Sparkplug B resources from the broker. The output is always in JSON format.

#### Get All Groups

Lists the IDs of all known Sparkplug B groups.

**Command:**
```sh
axonmq-cli spb get groups
```

**Example Output:**
```json
[
  "group"
]
```

---

#### Get All Nodes in a Group

Lists all nodes that have sent a birth certificate within a specific group.

**Command:**
```sh
axonmq-cli spb get nodes <group_id>
```

**Example:**
```sh
axonmq-cli spb get nodes group
```

**Example Output:**
```json
[
  {
    "metrics": [
      {
        "datatype": 11,
        "is_null": false,
        "name": "Node Control/Reboot",
        "properties": [],
        "stale": false,
        "timestamp": 1763972606818,
        "value": false
      },
      {
        "datatype": 8,
        "is_null": false,
        "name": "bdSeq",
        "properties": [],
        "stale": false,
        "timestamp": 1763972606818,
        "value": 0
      }
    ],
    "node_id": "node",
    "online": true,
    "setting": [],
    "templates": {},
    "timestamp": 1763972606818
  }
]
```

---

#### Get a Specific Device

Displays the details and all metrics for a single device.

**Command:**
```sh
axonmq-cli spb get device <group_id> <node_id> <device_id>
```

**Example:**
```sh
axonmq-cli spb get device group node sensor1
```

**Example Output:**
```json
{
  "device": "sensor1",
  "metrics": [
    {
      "datatype": 2,
      "is_null": false,
      "name": "temperature",
      "properties": [],
      "stale": false,
      "timestamp": 1763972819199,
      "value": 12
    }
  ],
  "online": true,
  "setting": [],
  "timestamp": 1763972819199
}
```
*(The commands `get group <id>`, `get node <group> <node>`, and `get devices <group> <node>` follow the same pattern.)*

---

### `spb set`

The `set` command is used to send commands to nodes or devices by updating the value of their metrics. This is done by sending a `PUT` request to the broker's API.

The metrics to be set are specified in `key=value` pairs.

#### Value Parsing Rules

The CLI automatically parses the value based on its format:
- **Number**: `Metric=123` or `Metric=45.6` will be sent as a JSON number.
- **Boolean**: `Metric=true` or `Metric=false` will be sent as a JSON boolean.
- **String**: To send a value as a string, enclose it in double quotes, like `Metric="Hello World"`. If a value is not a valid number or boolean and is not quoted, it will also be treated as a string (e.g., `Metric=Some_State`).

#### Set a Metric on a Device

Sends a command to a specific device.

**Command:**
```sh
axonmq-cli spb set device <group_id> <node_id> <device_id> <metric_name>=<value>...
```

**Example:**
```sh
axonmq-cli spb set device group node sensor1 temperature=13
```

**Example Output (Success):**
```json
{
  "details": [
    {
      "error": "success",
      "name": "temperature"
    }
  ]
}
```
