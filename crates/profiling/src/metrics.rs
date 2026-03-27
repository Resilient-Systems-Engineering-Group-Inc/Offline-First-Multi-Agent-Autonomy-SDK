//! Metrics collection (Prometheus style).

use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use common::types::AgentId;
use anyhow::Result;

static INITIALIZED: std::sync::Once = std::sync::Once::new();

/// Initialize the metrics subsystem.
pub fn init(service_name: &str, agent_id: AgentId) -> Result<()> {
    INITIALIZED.call_once(|| {
        let builder = PrometheusBuilder::new();
        // Install the global recorder.
        builder.install().expect("Failed to install metrics recorder");

        // Register static labels.
        metrics::describe_counter!(
            "events_total",
            "Total number of events",
            metrics::Unit::Count,
            "Total events"
        );
        metrics::describe_gauge!(
            "active_connections",
            "Number of active network connections",
            metrics::Unit::Count
        );
        metrics::describe_histogram!(
            "message_processing_duration_seconds",
            "Time spent processing a message",
            metrics::Unit::Seconds
        );
        metrics::describe_counter!(
            "messages_received_total",
            "Total messages received",
            metrics::Unit::Count
        );
        metrics::describe_counter!(
            "messages_sent_total",
            "Total messages sent",
            metrics::Unit::Count
        );
        metrics::describe_gauge!(
            "crdt_map_size",
            "Number of entries in the local CRDT map",
            metrics::Unit::Count
        );

        tracing::info!("Metrics initialized for {} (agent {:?})", service_name, agent_id);
    });
    Ok(())
}

/// Increment a counter with labels.
pub fn increment_counter(name: &'static str, labels: &[(&'static str, &str)]) {
    counter!(name, labels).increment(1);
}

/// Record a gauge value.
pub fn set_gauge(name: &'static str, value: f64, labels: &[(&'static str, &str)]) {
    gauge!(name, labels).set(value);
}

/// Record a histogram value.
pub fn record_histogram(name: &'static str, value: f64, labels: &[(&'static str, &str)]) {
    histogram!(name, labels).record(value);
}

/// Start a timer that records duration when dropped.
pub fn start_timer(name: &'static str, labels: &[(&'static str, &str)]) -> metrics::Timer {
    metrics::Timer::new(name, labels)
}

/// Expose metrics via an HTTP endpoint (blocking).
pub fn serve_metrics(addr: SocketAddr) -> Result<()> {
    std::thread::spawn(move || {
        let listener = std::net::TcpListener::bind(addr).unwrap();
        let mut stream = listener.incoming().next().unwrap().unwrap();
        let response = metrics_exporter_prometheus::encode_to_string().unwrap();
        let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
    });
    Ok(())
}