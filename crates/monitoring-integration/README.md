# Monitoring Integration

Integration with external monitoring systems (Prometheus, Grafana, Jaeger) for the Offline‑First Multi‑Agent Autonomy SDK.

## Features

- **Prometheus exporter**: Exposes metrics via HTTP endpoint `/metrics`.
- **Jaeger/OpenTelemetry tracing**: Distributed tracing with OTLP export.
- **Grafana dashboard automation**: Create and update dashboards via Grafana API.
- **Unified configuration**: Single configuration structure for all monitoring components.
- **Predefined metrics**: Common SDK metrics (tasks, CPU, memory, network latency, health).
- **Predefined dashboard templates**: Ready‑to‑use Grafana dashboards for agent and planning metrics.

## Usage

Add this crate to your `Cargo.toml`:

```toml
monitoring-integration = { path = "../crates/monitoring-integration" }
```

### Quick Start

```rust
use monitoring_integration::{MonitoringConfig, MonitoringManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = MonitoringConfig::default();
    let mut manager = MonitoringManager::new(config);
    manager.start().await?;
    // ... your application logic
    manager.stop().await?;
    Ok(())
}
```

### Configuration

You can customize the monitoring behavior via `MonitoringConfig`:

```rust
use monitoring_integration::{MonitoringConfig, PrometheusConfig, JaegerConfig, GrafanaConfig};
use std::net::SocketAddr;

let config = MonitoringConfig {
    prometheus: PrometheusConfig {
        enabled: true,
        bind_addr: "127.0.0.1:9090".parse().unwrap(),
        path: "/metrics".to_string(),
        collect_internal: true,
    },
    jaeger: JaegerConfig {
        enabled: true,
        endpoint: "http://localhost:4317".to_string(),
        service_name: "my‑agent".to_string(),
        sampling_rate: 0.5,
        timeout_secs: 5,
        protocol: "grpc".to_string(),
    },
    grafana: GrafanaConfig {
        enabled: true,
        base_url: "http://localhost:3000".to_string(),
        api_key: "your‑api‑key".to_string(),
        create_default_dashboards: true,
        folder_uid: None,
    },
};
```

### Metrics

The crate provides a set of predefined Prometheus metrics (see `prometheus::metrics`):

- `offline_first_tasks_created_total`
- `offline_first_tasks_assigned_total`
- `offline_first_tasks_completed_total`
- `offline_first_tasks_missed_deadline_total`
- `offline_first_pending_tasks`
- `offline_first_cpu_usage_percent`
- `offline_first_memory_usage_bytes`
- `offline_first_network_latency_seconds`
- `offline_first_health_status`

You can update them directly:

```rust
use monitoring_integration::prometheus::metrics;

metrics::TASKS_CREATED.inc();
metrics::CPU_USAGE_PERCENT.set(42.0);
metrics::NETWORK_LATENCY_SECONDS.observe(0.123);
```

### Tracing

If Jaeger is enabled, you can create spans and events:

```rust
use monitoring_integration::jaeger;

let span = jaeger::start_span("my_operation");
span.add_event("event_name", vec![KeyValue::new("key", "value")]);
span.end();
```

### Grafana Dashboards

The crate includes two predefined dashboard templates:

- `grafana::templates::default_agent_dashboard()` – general agent metrics.
- `grafana::templates::planning_dashboard()` – distributed planning metrics.

You can create them programmatically:

```rust
let client = GrafanaClient::new(base_url, api_key);
let dashboard = grafana::templates::default_agent_dashboard();
let response = client.create_dashboard(&dashboard).await?;
println!("Dashboard URL: {}", response.url);
```

## Example

Run the included example:

```bash
cargo run --example basic_monitoring
```

This starts a Prometheus exporter on `http://127.0.0.1:9090/metrics` and updates metrics every second for 10 seconds.

## Dependencies

- `prometheus` – metrics collection and exposition.
- `opentelemetry`, `opentelemetry‑otlp` – distributed tracing.
- `warp` – HTTP server for metrics endpoint.
- `reqwest` – HTTP client for Grafana API.
- `tracing`, `tracing‑opentelemetry`, `tracing‑subscriber` – structured logging and tracing integration.

## License

BSD‑3‑Clause