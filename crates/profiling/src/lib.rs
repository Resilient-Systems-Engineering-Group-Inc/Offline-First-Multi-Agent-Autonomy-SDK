//! Profiling and debugging tools for distributed multi‑agent scenarios.
//!
//! This crate provides:
//! - **Metrics**: Prometheus‑style metrics for agent activity, message counts, resource usage.
//! - **Distributed tracing**: OpenTelemetry‑compatible traces across agents.
//! - **State snapshots**: Dump and compare CRDT state across the swarm.
//! - **Debug endpoints**: HTTP server for live inspection.
//! - **Performance analysis**: Latency tracking, throughput measurement, alerting.
//! - **Distributed analysis**: Bottleneck detection, correlation analysis, anomaly detection.

pub mod metrics;
pub mod tracing;
pub mod snapshot;
pub mod debug_server;
pub mod performance_analysis;
pub mod distributed_analysis;

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

/// Re‑export distributed analysis types.
pub use distributed_analysis::{
    Bottleneck, MetricCorrelation, PerformanceAnomaly,
    BottleneckDetector, CorrelationAnalyzer, AnomalyDetector,
    DistributedPerformanceAnalyzer, PerformanceReport,
};

/// Initialize performance analysis subsystem and return a shared analyzer.
/// The analyzer will run background tasks for aggregation and alerting.
pub async fn init_performance_analysis() -> std::sync::Arc<PerformanceAnalyzer> {
    performance_analysis::init_performance_analysis().await
}

/// Initialize distributed performance analysis subsystem.
/// Returns a shared analyzer that can detect bottlenecks, correlations, and anomalies.
pub async fn init_distributed_analysis() -> std::sync::Arc<DistributedPerformanceAnalyzer> {
    std::sync::Arc::new(DistributedPerformanceAnalyzer::new())
}