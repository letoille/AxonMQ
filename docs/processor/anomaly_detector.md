# Anomaly Detector Processor Guide

## Overview

The `anomaly-detector` is a stateful processor designed for real-time anomaly detection in time-series data from MQTT messages. It can track values for thousands of unique time series independently and apply different detection strategies to identify unusual patterns or outliers.

When an anomaly is detected, the processor logs a detailed `WARN` level message. It does not modify or drop the message, allowing downstream processors to take further action if needed (in future designs that use metadata).

## Use Cases

- **Predictive Maintenance**: Detect when a machine's vibration or temperature deviates from its recent normal behavior, indicating a potential failure.
- **Environmental Monitoring**: Identify sudden spikes or drops in sensor readings (e.g., water quality pH, air pollution levels) that could signify an event.
- **Financial/IoT Metrics**: Monitor transaction volumes or data usage patterns to detect unusual activity.

## Configuration Parameters

The `anomaly_detector` processor is configured in your `config.toml` file.

```toml
[[processor]]
uuid = "your-detector-processor-uuid"
config = { type = "anomaly_detector", ... }
```

The `config` table has the following parameters:

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `type` | String | Yes | Must be `"anomaly_detector"`. |
| `value_selector` | String | Yes | A `minijinja` template expression to extract the numerical (`f64`) value to be monitored from the message. Typically a path into the JSON payload. |
| `series_id` | String | Yes | A `minijinja` template that generates a unique key to identify the time series. The processor maintains a separate state for each unique key. |
| `strategy` | Table | Yes | A table defining the detection algorithm and its parameters. |

**Note on `value_selector`**: This uses the `minijinja` engine. You can use simple dot notation like `"payload.temperature"` or more complex expressions like `"payload.voltage * payload.current"`.

**Note on `series_id`**: This allows the processor to track state for many devices at once. A common value is `"{{ client_id }}"` to track each device separately.

## Detection Strategies

You must specify one of the following strategies in the `strategy` table.

### `threshold` Strategy

This is a simple, stateless strategy that checks if a value falls outside a fixed range.

- **`type`**: Must be `"threshold"`.
- **Parameters**:
    - `min` (Number): The minimum acceptable value.
    - `max` (Number): The maximum acceptable value.

**Example:**
```toml
strategy = {
    type = "threshold",
    min = 0.0,
    max = 100.0
}
```

### `moving_average` Strategy

This is a stateful strategy that detects values that deviate significantly from the recent trend of a time series.

- **`type`**: Must be `"moving_average"`.
- **Parameters**:
    - `window_size` (Integer): The number of recent data points to keep in the moving window for calculation.
    - `deviation_factor` (Number): The number of standard deviations (σ) away from the moving average that a point must be to be considered an anomaly.

**Example:**
```toml
strategy = {
    type = "moving_average",
    window_size = 50,      # Use the last 50 points
    deviation_factor = 3.0 # Flag anything outside 3 standard deviations
}
```

## Full Example

This configuration sets up a processor to detect anomalies in pressure readings from a pump, tracking each pump by its `client_id`.

```toml
[[router]]
topic = "sensors/pump-007/pressure"
chain = ["pressure_anomaly_detection_chain"]

[[chain]]
name = "pressure_anomaly_detection_chain"
processors = ["a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6"]
delivery = true

[[processor]]
uuid = "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6"
config = {
    type = "anomaly_detector",
    value_selector = "payload.pressure",
    series_id = "{{ client_id }}",
    strategy = {
        type = "moving_average",
        window_size = 20,
        deviation_factor = 2.5
    }
}
```
