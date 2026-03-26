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