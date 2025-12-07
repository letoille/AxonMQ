# AxonMQ RESTful API Reference

## Introduction

This document provides a reference for the AxonMQ RESTful API. The API is designed to follow standard REST principles, using resource-oriented URLs and standard HTTP verbs.

All API endpoints are prefixed with `/api/v1`.

The API provides access to the real-time state of the services running within the AxonMQ broker, with an initial focus on the `SparkplugService`.

**Note**: The current API is unauthenticated and is intended for use in trusted environments. Authentication and authorization will be added in a future release.

### Error Responses

API errors are returned with an appropriate HTTP status code (e.g., `404 Not Found`, `500 Internal Server Error`) and a standard JSON body:

```json
{
  "result": "Error message text...",
  "details": []
}
```

## Sparkplug B Service API

All Sparkplug B related endpoints are under the `/api/v1/services/sparkplug_b` path.

---

### `GET` Endpoints (Read Data)

#### Get All Groups

Returns a list of all currently active Group IDs.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups`
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups
  ```
- **Example Response** (`200 OK`):
  ```json
  [
    "group"
  ]
  ```

#### Get a Specific Group

Returns the ID of a specific group if it exists.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}`
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/group
  ```
- **Example Response** (`200 OK`):
  ```json
  "group"
  ```

---

#### Get All Nodes in a Group

Returns a list of all nodes within a specific Group.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes`
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/group
  ```
- **Example Response** (`200 OK`):
  ```json
  [
    {
      "metrics": [
        { "name": "Node Control/Reboot", "value": false, ... },
        { "name": "bdSeq", "value": 0, ... }
      ],
      "node_id": "node",
      "online": true,
      "setting": [],
      "templates": {},
      "timestamp": 1763972606818
    }
  ]
  ```

#### Get a Specific Node

Returns detailed information for a single Node.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}`
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/group/nodes/node
  ```
- **Example Response** (`200 OK`):
  ```json
  {
    "metrics": [
      { "name": "Node Control/Reboot", "value": false, ... }
    ],
    "node_id": "node",
    "online": true,
    "setting": [],
    "templates": {},
    "timestamp": 1763972606818
  }
  ```

---

#### Get All Devices on a Node

Returns a list of all devices attached to a specific Node.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}/devices`
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/group/nodes/node/devices
  ```
- **Example Response** (`200 OK`):
  ```json
  [
    {
      "device": "mb1",
      "metrics": [
        { "name": "g1/tag1", "value": 123, ... }
      ],
      "online": true,
      "setting": [],
      "timestamp": 1763972804198
    }
  ]
  ```

#### Get a Specific Device

Returns detailed information for a single Device.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}/devices/{device_id}`
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/group/nodes/node/devices/mb1
  ```
- **Example Response** (`200 OK`):
  ```json
  {
    "device": "mb1",
    "metrics": [
      { "name": "g1/tag1", "value": 123, ... }
    ],
    "online": true,
    "setting": [],
    "timestamp": 1763972819199
  }
  ```

---

### `PUT` Endpoints (Write Commands)

#### Set Metrics on a Device

Issues a command to a device by setting the value of one or more metrics.

- **Method**: `PUT`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}/devices/{device_id}`
- **Request Body**: A JSON array of Key-Value objects.
  ```json
  [
    {
      "name": "metric_name",
      "value": "some_value"
    },
    {
      "name": "another_metric",
      "value": 123.45
    }
  ]
  ```
- **Example Request**:
  ```bash
  curl -X PUT -H "Content-Type: application/json" \
    -d '[{"name": "g1/tag1", "value": 123}]' \
    http://localhost:1107/api/v1/services/sparkplug_b/groups/group/nodes/node/devices/mb1
  ```
- **Example Success Response** (`200 OK`):
  ```json
  {
    "details": [
      {
        "error": "success",
        "name": "g1/tag1"
      }
    ]
  }
  ```
- **Example Failure Response** (`200 OK` with error details):
  ```json
  {
    "details": [],
    "result": "SparkPlug B Error: Node Not Found"
  }
  ```

*(Setting metrics on a Node follows the same pattern at the `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}` endpoint.)*
