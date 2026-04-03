//! Profiling and debugging tools for distributed multi‑agent scenarios.
//!
//! This crate provides:
//! - **Metrics**: Prometheus‑style metrics for agent activity, message counts, resource usage.
//! - **Distributed tracing**: OpenTelemetry‑compatible traces across agents.
//! - **State snapshots**: Dump and compare CRDT state across the swarm.
//! - **Debug endpoints**: HTTP server for live inspection.

pub mod metrics;
pub mod tracing;
pub mod snapshot;
pub mod debug_server;
pub mod performance_analysis;

/// Re‑export common types for convenience.
pub use common::types::AgentId;

/// Initialize the profiling subsystem (metrics, tracing, debug server).
/// Call this once at the start of your application.
pub async fn init(service_name: &str, agent_id: AgentId) -> anyhow::Result<()> {
    metrics::init(service_name, agent_id)?;
    tracing::init(service_name, agent_id)?;
    tokio::spawn(debug_server::run(([127, 0, 0, 1], 9090).into()));
    Ok(())
}

/// Record a generic event for debugging.
pub fn record_event(event: &str, metadata: &[(&str, &str)]) {
    tracing::record_event(event, metadata);
    metrics::increment_counter("events_total", &[("event", event)]);
}

/// Re‑export performance analysis types.
pub use performance_analysis::{
    PerformanceAnalyzer, LatencyTracker, ThroughputMeter, ResourceCorrelator,
    AlertThreshold, Condition, Severity, Alert, LatencyStats,
};

/// Initialize performance analysis subsystem and return a shared analyzer.
/// The analyzer will run background tasks for aggregation and alerting.
pub async fn init_performance_analysis() -> std::sync::Arc<PerformanceAnalyzer> {
    performance_analysis::init_performance_analysis().await
}