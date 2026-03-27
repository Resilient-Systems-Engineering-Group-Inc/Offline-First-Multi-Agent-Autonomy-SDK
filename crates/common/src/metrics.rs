//! Prometheus metrics for the SDK.

use lazy_static::lazy_static;
use prometheus::{register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram, Encoder, TextEncoder};
use std::net::SocketAddr;
use tokio::task;
use warp::{Filter, Reply};

lazy_static! {
    /// Total number of messages sent via mesh transport.
    pub static ref MESSAGES_SENT: Counter = register_counter!(
        "offline_first_messages_sent_total",
        "Total number of messages sent"
    ).unwrap();

    /// Total number of messages received.
    pub static ref MESSAGES_RECEIVED: Counter = register_counter!(
        "offline_first_messages_received_total",
        "Total number of messages received"
    ).unwrap();

    /// Number of currently connected peers.
    pub static ref CONNECTED_PEERS: Gauge = register_gauge!(
        "offline_first_connected_peers",
        "Number of currently connected peers"
    ).unwrap();

    /// CRDT map size (number of keys).
    pub static ref CRDT_MAP_SIZE: Gauge = register_gauge!(
        "offline_first_crdt_map_size",
        "Number of keys in the CRDT map"
    ).unwrap();

    /// Consensus rounds started.
    pub static ref CONSENSUS_ROUNDS_STARTED: Counter = register_counter!(
        "offline_first_consensus_rounds_started_total",
        "Total number of consensus rounds started"
    ).unwrap();

    /// Consensus rounds completed successfully.
    pub static ref CONSENSUS_ROUNDS_COMPLETED: Counter = register_counter!(
        "offline_first_consensus_rounds_completed_total",
        "Total number of consensus rounds completed"
    ).unwrap();

    /// Message processing latency histogram.
    pub static ref MESSAGE_PROCESSING_LATENCY: Histogram = register_histogram!(
        "offline_first_message_processing_latency_seconds",
        "Latency of processing a message in seconds",
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();
}
    /// Number of tasks created in the distributed planner.
    pub static ref TASKS_CREATED: Counter = register_counter!(
        "offline_first_tasks_created_total",
        "Total number of tasks created"
    ).unwrap();

    /// Number of tasks assigned to agents.
    pub static ref TASKS_ASSIGNED: Counter = register_counter!(
        "offline_first_tasks_assigned_total",
        "Total number of tasks assigned"
    ).unwrap();

    /// Number of tasks completed.
    pub static ref TASKS_COMPLETED: Counter = register_counter!(
        "offline_first_tasks_completed_total",
        "Total number of tasks completed"
    ).unwrap();

    /// Number of tasks missed deadline.
    pub static ref TASKS_MISSED_DEADLINE: Counter = register_counter!(
        "offline_first_tasks_missed_deadline_total",
        "Total number of tasks that missed their deadline"
    ).unwrap();

    /// Current number of pending tasks.
    pub static ref PENDING_TASKS: Gauge = register_gauge!(
        "offline_first_pending_tasks",
        "Current number of pending tasks"
    ).unwrap();

    /// Resource usage gauges (CPU, memory, etc.)
    pub static ref CPU_USAGE_PERCENT: Gauge = register_gauge!(
        "offline_first_cpu_usage_percent",
        "CPU usage percent of the agent"
    ).unwrap();

    pub static ref MEMORY_USAGE_BYTES: Gauge = register_gauge!(
        "offline_first_memory_usage_bytes",
        "Memory usage in bytes"
    ).unwrap();

    /// Network latency histogram between agents.
    pub static ref NETWORK_LATENCY_SECONDS: Histogram = register_histogram!(
        "offline_first_network_latency_seconds",
        "Network latency between agents in seconds",
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0]
    ).unwrap();

    /// Health check status (1 = healthy, 0 = unhealthy).
    pub static ref HEALTH_STATUS: Gauge = register_gauge!(
        "offline_first_health_status",
        "Health status of the agent (1 healthy, 0 unhealthy)"
    ).unwrap();

/// Start a Prometheus metrics HTTP server on the given address.
pub async fn start_metrics_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let route = warp::path("metrics").map(|| {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        warp::reply::with_header(buffer, "Content-Type", "text/plain; version=0.0.4")
    });

    let (_, server) = warp::serve(route).bind_with_graceful_shutdown(addr, async {
        tokio::signal::ctrl_c().await.ok();
    });

    task::spawn(server);
    Ok(())
}

/// Increment the messages sent counter.
pub fn inc_messages_sent() {
    MESSAGES_SENT.inc();
}

/// Increment the messages received counter.
pub fn inc_messages_received() {
    MESSAGES_RECEIVED.inc();
}

/// Set the connected peers gauge.
pub fn set_connected_peers(count: usize) {
    CONNECTED_PEERS.set(count as f64);
}

/// Set the CRDT map size gauge.
pub fn set_crdt_map_size(size: usize) {
    CRDT_MAP_SIZE.set(size as f64);
}

/// Increment consensus rounds started.
pub fn inc_consensus_rounds_started() {
    CONSENSUS_ROUNDS_STARTED.inc();
}

/// Increment consensus rounds completed.
pub fn inc_consensus_rounds_completed() {
    CONSENSUS_ROUNDS_COMPLETED.inc();
}

/// Observe message processing latency.
pub fn observe_message_processing_latency(seconds: f64) {
    MESSAGE_PROCESSING_LATENCY.observe(seconds);
}
/// Increment tasks created counter.
pub fn inc_tasks_created() {
    TASKS_CREATED.inc();
}

/// Increment tasks assigned counter.
pub fn inc_tasks_assigned() {
    TASKS_ASSIGNED.inc();
}

/// Increment tasks completed counter.
pub fn inc_tasks_completed() {
    TASKS_COMPLETED.inc();
}

/// Increment tasks missed deadline counter.
pub fn inc_tasks_missed_deadline() {
    TASKS_MISSED_DEADLINE.inc();
}

/// Set pending tasks gauge.
pub fn set_pending_tasks(count: usize) {
    PENDING_TASKS.set(count as f64);
}

/// Set CPU usage percent.
pub fn set_cpu_usage_percent(percent: f64) {
    CPU_USAGE_PERCENT.set(percent);
}

/// Set memory usage in bytes.
pub fn set_memory_usage_bytes(bytes: u64) {
    MEMORY_USAGE_BYTES.set(bytes as f64);
}

/// Observe network latency.
pub fn observe_network_latency(seconds: f64) {
    NETWORK_LATENCY_SECONDS.observe(seconds);
}

/// Set health status (true = healthy, false = unhealthy).
pub fn set_health_status(healthy: bool) {
    HEALTH_STATUS.set(if healthy { 1.0 } else { 0.0 });
}