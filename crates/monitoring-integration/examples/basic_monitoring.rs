//! Basic example of monitoring integration.

use monitoring_integration::{
    MonitoringConfig, MonitoringManager, PrometheusConfig, JaegerConfig, GrafanaConfig,
};
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable logging
    env_logger::init();

    // Configuration
    let config = MonitoringConfig {
        prometheus: PrometheusConfig {
            enabled: true,
            bind_addr: "127.0.0.1:9090".parse().unwrap(),
            path: "/metrics".to_string(),
            collect_internal: true,
        },
        jaeger: JaegerConfig {
            enabled: false, // Set to true if you have a Jaeger/OTLP endpoint
            endpoint: "http://localhost:4317".to_string(),
            service_name: "example-agent".to_string(),
            sampling_rate: 1.0,
            timeout_secs: 5,
            protocol: "grpc".to_string(),
        },
        grafana: GrafanaConfig {
            enabled: false, // Set to true if you have Grafana API key
            base_url: "http://localhost:3000".to_string(),
            api_key: "".to_string(),
            create_default_dashboards: false,
            folder_uid: None,
        },
    };

    // Create and start monitoring manager
    let mut manager = MonitoringManager::new(config);
    manager.start().await?;

    println!("Monitoring started. Prometheus metrics at http://127.0.0.1:9090/metrics");
    println!("Press Ctrl+C to stop.");

    // Simulate some activity that updates metrics
    for i in 0..10 {
        // Update some metrics (these are defined in prometheus::metrics)
        monitoring_integration::prometheus::metrics::TASKS_CREATED.inc();
        monitoring_integration::prometheus::metrics::PENDING_TASKS.set(i as f64);
        monitoring_integration::prometheus::metrics::CPU_USAGE_PERCENT.set(20.0 + (i as f64 * 5.0));
        monitoring_integration::prometheus::metrics::NETWORK_LATENCY_SECONDS.observe(0.01 * i as f64);

        // Record a trace if Jaeger is enabled
        if let Some(tracer) = manager.jaeger_tracer() {
            let span = tracer.span("example_loop", vec![]);
            span.add_event("iteration", vec![opentelemetry::KeyValue::new("iteration", i)]);
            span.end();
        }

        println!("Iteration {} completed", i);
        sleep(Duration::from_secs(1)).await;
    }

    // Stop monitoring
    manager.stop().await?;
    println!("Monitoring stopped.");

    Ok(())
}