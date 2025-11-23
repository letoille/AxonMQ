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
  "error": "A message describing the error."
}
```

## Sparkplug B Service API

All Sparkplug B related endpoints are under the `/api/v1/services/sparkplug_b` path.

---

### Groups

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
    "MyFactory",
    "AnotherSite"
  ]
  ```

---

### Nodes

#### Get All Nodes in a Group

Returns a list of all nodes within a specific Group.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes`
- **Path Parameters**:
  - `group_id` (string, required): The ID of the group.
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/MyFactory/nodes
  ```
- **Example Response** (`200 OK`):
  ```json
  [
    {
      "node_id": "Node-001",
      "online": true,
      "timestamp": 1678886400000,
      "setting": [],
      "metrics": [
        { "name": "Node Control/Rebirth", "value": false, ... }
      ],
      "templates": {}
    }
  ]
  ```

#### Get a Specific Node

Returns detailed information for a single Node, including its metrics and a list of its devices.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}`
- **Path Parameters**:
  - `group_id` (string, required): The ID of the group.
  - `node_id` (string, required): The ID of the node.
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/MyFactory/nodes/Node-001
  ```
- **Example Response** (`200 OK`):
  ```json
  {
    "node_id": "Node-001",
    "online": true,
    "timestamp": 1678886400000,
    "setting": [],
    "metrics": [
      { "name": "Node Control/Rebirth", "value": false, ... }
    ],
    "templates": {
      "PumpTemplate": { ... }
    }
  }
  ```

---

### Devices

#### Get All Devices on a Node

Returns a list of all devices attached to a specific Node.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}/devices`
- **Path Parameters**:
  - `group_id` (string, required): The ID of the group.
  - `node_id` (string, required): The ID of the node.
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/MyFactory/nodes/Node-001/devices
  ```
- **Example Response** (`200 OK`):
  ```json
  [
    {
      "device": "Pump-01",
      "online": true,
      "timestamp": 1678886400123,
      "setting": [
        { "name": "config/setpoint", "value": 120.5, ... }
      ],
      "metrics": [
        { "name": "data/pressure", "value": 121.2, ... }
      ]
    }
  ]
  ```

#### Get a Specific Device

Returns detailed information for a single Device, including all its current metrics.

- **Method**: `GET`
- **Endpoint**: `/api/v1/services/sparkplug_b/groups/{group_id}/nodes/{node_id}/devices/{device_id}`
- **Path Parameters**:
  - `group_id` (string, required): The ID of the group.
  - `node_id` (string, required): The ID of the node.
  - `device_id` (string, required): The ID of the device.
- **Example Request**:
  ```bash
  curl http://localhost:1107/api/v1/services/sparkplug_b/groups/MyFactory/nodes/Node-001/devices/Pump-01
  ```
- **Example Response** (`200 OK`):
  ```json
  {
    "device": "Pump-01",
    "online": true,
    "timestamp": 1678886400123,
    "setting": [
      { "name": "config/setpoint", "value": 120.5, ... }
    ],
    "metrics": [
      { "name": "data/pressure", "value": 121.2, ... }
    ]
  }
  ```
