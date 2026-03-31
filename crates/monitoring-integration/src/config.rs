//! Configuration for monitoring integration.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Overall monitoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Prometheus exporter configuration.
    pub prometheus: PrometheusConfig,
    /// Jaeger (OpenTelemetry) configuration.
    pub jaeger: JaegerConfig,
    /// Grafana configuration.
    pub grafana: GrafanaConfig,
}

/// Prometheus exporter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Enable Prometheus exporter.
    pub enabled: bool,
    /// Bind address for the HTTP server.
    pub bind_addr: SocketAddr,
    /// Path for metrics endpoint (default "/metrics").
    pub path: String,
    /// Collect internal metrics from the SDK.
    pub collect_internal: bool,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_addr: "127.0.0.1:9090".parse().unwrap(),
            path: "/metrics".to_string(),
            collect_internal: true,
        }
    }
}

/// Jaeger (OpenTelemetry) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaegerConfig {
    /// Enable Jaeger tracing.
    pub enabled: bool,
    /// OTLP endpoint (e.g., "http://localhost:4317").
    pub endpoint: String,
    /// Service name.
    pub service_name: String,
    /// Sampling rate (0.0 to 1.0).
    pub sampling_rate: f64,
    /// Timeout for export in seconds.
    pub timeout_secs: u64,
    /// Protocol (grpc or http/protobuf).
    pub protocol: String,
}

impl Default for JaegerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:4317".to_string(),
            service_name: "offline-first-agent".to_string(),
            sampling_rate: 1.0,
            timeout_secs: 5,
            protocol: "grpc".to_string(),
        }
    }
}

/// Grafana configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaConfig {
    /// Enable Grafana dashboard auto‑creation.
    pub enabled: bool,
    /// Grafana base URL (e.g., "http://localhost:3000").
    pub base_url: String,
    /// API key (bearer token).
    pub api_key: String,
    /// Automatically create default dashboards on startup.
    pub create_default_dashboards: bool,
    /// Dashboard folder UID (optional).
    pub folder_uid: Option<String>,
}

impl Default for GrafanaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://localhost:3000".to_string(),
            api_key: "".to_string(),
            create_default_dashboards: false,
            folder_uid: None,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            prometheus: PrometheusConfig::default(),
            jaeger: JaegerConfig::default(),
            grafana: GrafanaConfig::default(),
        }
    }
}