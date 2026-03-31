//! Integration with external monitoring systems (Prometheus, Grafana, Jaeger).
//!
//! This crate provides a unified interface to export metrics, traces, and dashboards
//! to popular monitoring stacks.
//!
//! # Quick Start
//!
//! ```no_run
//! use monitoring_integration::{MonitoringManager, MonitoringConfig};
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = MonitoringConfig::default();
//!     let mut manager = MonitoringManager::new(config);
//!     manager.start().await?;
//!     // ... run your application
//!     manager.stop().await?;
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod grafana;
pub mod jaeger;
pub mod prometheus;

pub use config::*;
pub use error::*;
pub use grafana::*;
pub use jaeger::*;
pub use prometheus::*;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// High‑level manager that orchestrates all monitoring integrations.
pub struct MonitoringManager {
    config: MonitoringConfig,
    prometheus_exporter: Option<prometheus::PrometheusExporter>,
    jaeger_tracer: Option<jaeger::JaegerTracer>,
    grafana_client: Option<grafana::GrafanaClient>,
    started: bool,
}

impl MonitoringManager {
    /// Create a new monitoring manager with the given configuration.
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            prometheus_exporter: None,
            jaeger_tracer: None,
            grafana_client: None,
            started: false,
        }
    }

    /// Start all enabled monitoring integrations.
    pub async fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        // Start Prometheus exporter
        if self.config.prometheus.enabled {
            let mut exporter = prometheus::PrometheusExporter::new(self.config.prometheus.bind_addr);
            if self.config.prometheus.collect_internal {
                prometheus::metrics::register_all(exporter.registry())?;
            }
            exporter.start().await?;
            self.prometheus_exporter = Some(exporter);
            log::info!("Prometheus exporter started");
        }

        // Start Jaeger tracer
        if self.config.jaeger.enabled {
            let mut tracer = jaeger::JaegerTracer::new(jaeger::JaegerConfig {
                endpoint: self.config.jaeger.endpoint.clone(),
                service_name: self.config.jaeger.service_name.clone(),
                sampling_rate: self.config.jaeger.sampling_rate,
                timeout_secs: self.config.jaeger.timeout_secs,
                protocol: match self.config.jaeger.protocol.as_str() {
                    "grpc" => opentelemetry_otlp::Protocol::Grpc,
                    "http" => opentelemetry_otlp::Protocol::HttpBinary,
                    _ => opentelemetry_otlp::Protocol::Grpc,
                },
            });
            tracer.init()?;
            self.jaeger_tracer = Some(tracer);
            log::info!("Jaeger tracer started");
        }

        // Create Grafana client
        if self.config.grafana.enabled {
            let client = grafana::GrafanaClient::new(
                self.config.grafana.base_url.clone(),
                self.config.grafana.api_key.clone(),
            );
            if self.config.grafana.create_default_dashboards {
                // Create default dashboards
                let dashboard = grafana::templates::default_agent_dashboard();
                match client.create_dashboard(&dashboard).await {
                    Ok(resp) => log::info!("Default dashboard created: {}", resp.url),
                    Err(e) => log::warn!("Failed to create default dashboard: {}", e),
                }
                let planning = grafana::templates::planning_dashboard();
                match client.create_dashboard(&planning).await {
                    Ok(resp) => log::info!("Planning dashboard created: {}", resp.url),
                    Err(e) => log::warn!("Failed to create planning dashboard: {}", e),
                }
            }
            self.grafana_client = Some(client);
            log::info!("Grafana client initialized");
        }

        self.started = true;
        log::info!("Monitoring manager started");
        Ok(())
    }

    /// Stop all monitoring integrations.
    pub async fn stop(&mut self) -> Result<()> {
        if !self.started {
            return Ok(());
        }

        if let Some(mut exporter) = self.prometheus_exporter.take() {
            exporter.stop().await?;
            log::info!("Prometheus exporter stopped");
        }

        if let Some(tracer) = self.jaeger_tracer.take() {
            tracer.shutdown();
            log::info!("Jaeger tracer stopped");
        }

        // Grafana client doesn't need shutdown.

        self.started = false;
        log::info!("Monitoring manager stopped");
        Ok(())
    }

    /// Get a reference to the Prometheus exporter (if enabled).
    pub fn prometheus_exporter(&self) -> Option<&prometheus::PrometheusExporter> {
        self.prometheus_exporter.as_ref()
    }

    /// Get a reference to the Jaeger tracer (if enabled).
    pub fn jaeger_tracer(&self) -> Option<&jaeger::JaegerTracer> {
        self.jaeger_tracer.as_ref()
    }

    /// Get a reference to the Grafana client (if enabled).
    pub fn grafana_client(&self) -> Option<&grafana::GrafanaClient> {
        self.grafana_client.as_ref()
    }

    /// Check if monitoring is started.
    pub fn is_started(&self) -> bool {
        self.started
    }
}

impl Drop for MonitoringManager {
    fn drop(&mut self) {
        if self.started {
            log::warn!("MonitoringManager dropped without calling stop()");
            // Attempt to stop synchronously (not ideal but better than nothing)
            let mut manager = std::mem::replace(self, Self::new(MonitoringConfig::default()));
            let _ = tokio::runtime::Handle::try_current().map(|handle| {
                handle.block_on(async {
                    let _ = manager.stop().await;
                })
            });
        }
    }
}

/// Trait for components that can expose monitoring metrics.
#[async_trait]
pub trait Monitorable {
    /// Update internal metrics (called periodically).
    async fn update_metrics(&self);
    /// Record a trace event.
    fn record_trace(&self, name: &str, attributes: Vec<opentelemetry::KeyValue>);
}

/// Convenience function to start a global monitoring manager.
pub async fn start_global_monitoring(config: MonitoringConfig) -> Result<Arc<RwLock<MonitoringManager>>> {
    let mut manager = MonitoringManager::new(config);
    manager.start().await?;
    Ok(Arc::new(RwLock::new(manager)))
}