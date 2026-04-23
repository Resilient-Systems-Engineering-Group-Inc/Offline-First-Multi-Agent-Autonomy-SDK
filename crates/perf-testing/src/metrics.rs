//! Performance metrics collection and monitoring.

use anyhow::Result;
use prometheus::{IntCounter, IntGauge, HistogramOpts, Histogram, Registry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Metrics collector.
pub struct MetricsCollector {
    registry: Registry,
    request_counter: IntCounter,
    response_time: Histogram,
    active_connections: IntGauge,
    error_counter: IntCounter,
}

impl MetricsCollector {
    /// Create new metrics collector.
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        let request_counter = IntCounter::new(
            "sdk_requests_total",
            "Total number of requests",
        )?;

        let response_time = Histogram::with_opts(HistogramOpts::new(
            "sdk_response_time_ms",
            "Response time in milliseconds",
        ))?;

        let active_connections = IntGauge::new(
            "sdk_active_connections",
            "Number of active connections",
        )?;

        let error_counter = IntCounter::new(
            "sdk_errors_total",
            "Total number of errors",
        )?;

        registry.register(Box::new(request_counter.clone()))?;
        registry.register(Box::new(response_time.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(error_counter.clone()))?;

        Ok(Self {
            registry,
            request_counter,
            response_time,
            active_connections,
            error_counter,
        })
    }

    /// Record request.
    pub fn record_request(&self) {
        self.request_counter.inc();
    }

    /// Record response time.
    pub fn record_response_time(&self, duration_ms: f64) {
        self.response_time.observe(duration_ms);
    }

    /// Record error.
    pub fn record_error(&self) {
        self.error_counter.inc();
    }

    /// Set active connections.
    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count);
    }

    /// Get metrics in Prometheus format.
    pub fn get_metrics(&self) -> Result<String> {
        let mut buffer = Vec::new();
        prometheus::encode_to_vec(&self.registry, &mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// Get request count.
    pub fn get_request_count(&self) -> u64 {
        self.request_counter.get()
    }

    /// Get error count.
    pub fn get_error_count(&self) -> u64 {
        self.error_counter.get()
    }

    /// Get response time histogram.
    pub fn get_response_time_stats(&self) -> ResponseTimeStats {
        let summary = self.response_time.clone().get_summary();
        
        ResponseTimeStats {
            count: summary.sample_count() as i64,
            sum: summary.sample_sum(),
            avg: if summary.sample_count() > 0 {
                summary.sample_sum() / summary.sample_count() as f64
            } else {
                0.0
            },
            p50: summary.quantile().iter()
                .find(|q| q.quantile() == 0.5)
                .map(|q| q.value())
                .unwrap_or(0.0),
            p95: summary.quantile().iter()
                .find(|q| q.quantile() == 0.95)
                .map(|q| q.value())
                .unwrap_or(0.0),
            p99: summary.quantile().iter()
                .find(|q| q.quantile() == 0.99)
                .map(|q| q.value())
                .unwrap_or(0.0),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Response time statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeStats {
    pub count: i64,
    pub sum: f64,
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub requests_total: u64,
    pub errors_total: u64,
    pub active_connections: i64,
    pub response_time: ResponseTimeStats,
    pub requests_per_second: f64,
    pub error_rate_percent: f64,
}

impl PerformanceMetrics {
    pub fn from_collector(collector: &MetricsCollector) -> Self {
        let response_time = collector.get_response_time_stats();
        let requests = collector.get_request_count();
        let errors = collector.get_error_count();

        Self {
            timestamp: chrono::Utc::now(),
            requests_total: requests,
            errors_total: errors,
            active_connections: collector.active_connections.get(),
            response_time,
            requests_per_second: requests as f64 / 60.0, // Assuming 1 minute window
            error_rate_percent: if requests > 0 {
                (errors as f64 / requests as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new().unwrap();

        collector.record_request();
        collector.record_response_time(50.0);
        collector.record_error();
        collector.set_active_connections(10);

        assert_eq!(collector.get_request_count(), 1);
        assert_eq!(collector.get_error_count(), 1);
        assert_eq!(collector.active_connections.get(), 10);

        let metrics = collector.get_metrics().unwrap();
        assert!(!metrics.is_empty());
    }
}
