//! Prometheus metrics exporter.

use crate::error::{MonitoringError, Result};
use prometheus::{
    self, register_counter, register_gauge, register_histogram, Counter, Encoder, Gauge, Histogram,
    TextEncoder,
};
use std::net::SocketAddr;
use tokio::task;
use warp::Filter;

/// Prometheus metrics exporter.
pub struct PrometheusExporter {
    /// Address to bind the HTTP server.
    bind_addr: SocketAddr,
    /// Custom metrics registry (optional).
    registry: prometheus::Registry,
    /// HTTP server handle.
    server_handle: Option<task::JoinHandle<()>>,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter.
    pub fn new(bind_addr: SocketAddr) -> Self {
        let registry = prometheus::Registry::new();
        Self {
            bind_addr,
            registry,
            server_handle: None,
        }
    }

    /// Add a custom metric to the registry.
    pub fn register_metric<M: prometheus::Collector + 'static>(
        &mut self,
        metric: M,
    ) -> Result<()> {
        self.registry
            .register(Box::new(metric))
            .map_err(MonitoringError::Prometheus)
    }

    /// Start the HTTP server that exposes `/metrics`.
    pub async fn start(&mut self) -> Result<()> {
        let registry = self.registry.clone();
        let metrics_route = warp::path("metrics").map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = vec![];
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });

        let (addr, server) = warp::serve(metrics_route).bind_ephemeral(self.bind_addr);
        log::info!("Prometheus exporter listening on http://{}", addr);

        let handle = task::spawn(async move {
            server.await;
        });
        self.server_handle = Some(handle);
        Ok(())
    }

    /// Stop the HTTP server.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        Ok(())
    }
}

/// Predefined metrics for the SDK.
pub mod metrics {
    use super::*;

    lazy_static::lazy_static! {
        /// Total number of tasks created.
        pub static ref TASKS_CREATED: Counter = register_counter!(
            "offline_first_tasks_created_total",
            "Total number of tasks created"
        ).unwrap();

        /// Total number of tasks assigned.
        pub static ref TASKS_ASSIGNED: Counter = register_counter!(
            "offline_first_tasks_assigned_total",
            "Total number of tasks assigned"
        ).unwrap();

        /// Total number of tasks completed.
        pub static ref TASKS_COMPLETED: Counter = register_counter!(
            "offline_first_tasks_completed_total",
            "Total number of tasks completed"
        ).unwrap();

        /// Total number of tasks that missed their deadline.
        pub static ref TASKS_MISSED_DEADLINE: Counter = register_counter!(
            "offline_first_tasks_missed_deadline_total",
            "Total number of tasks that missed their deadline"
        ).unwrap();

        /// Number of pending tasks.
        pub static ref PENDING_TASKS: Gauge = register_gauge!(
            "offline_first_pending_tasks",
            "Number of pending tasks"
        ).unwrap();

        /// CPU usage percentage.
        pub static ref CPU_USAGE_PERCENT: Gauge = register_gauge!(
            "offline_first_cpu_usage_percent",
            "CPU usage percentage"
        ).unwrap();

        /// Memory usage in bytes.
        pub static ref MEMORY_USAGE_BYTES: Gauge = register_gauge!(
            "offline_first_memory_usage_bytes",
            "Memory usage in bytes"
        ).unwrap();

        /// Network latency histogram.
        pub static ref NETWORK_LATENCY_SECONDS: Histogram = register_histogram!(
            "offline_first_network_latency_seconds",
            "Network latency in seconds",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
        ).unwrap();

        /// Health status (1 = healthy, 0 = unhealthy).
        pub static ref HEALTH_STATUS: Gauge = register_gauge!(
            "offline_first_health_status",
            "Health status (1 = healthy, 0 = unhealthy)"
        ).unwrap();
    }

    /// Register all predefined metrics with a given registry.
    pub fn register_all(registry: &prometheus::Registry) -> Result<()> {
        registry
            .register(Box::new(TASKS_CREATED.clone()))
            .and_then(|_| registry.register(Box::new(TASKS_ASSIGNED.clone())))
            .and_then(|_| registry.register(Box::new(TASKS_COMPLETED.clone())))
            .and_then(|_| registry.register(Box::new(TASKS_MISSED_DEADLINE.clone())))
            .and_then(|_| registry.register(Box::new(PENDING_TASKS.clone())))
            .and_then(|_| registry.register(Box::new(CPU_USAGE_PERCENT.clone())))
            .and_then(|_| registry.register(Box::new(MEMORY_USAGE_BYTES.clone())))
            .and_then(|_| registry.register(Box::new(NETWORK_LATENCY_SECONDS.clone())))
            .and_then(|_| registry.register(Box::new(HEALTH_STATUS.clone())))
            .map_err(MonitoringError::Prometheus)
    }
}