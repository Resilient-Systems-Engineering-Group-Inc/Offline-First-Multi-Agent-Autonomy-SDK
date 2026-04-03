//! Quality metrics collection and reporting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use prometheus::{Counter, Gauge, Histogram, Registry};

use crate::error::Result;

/// Quality metric type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityMetric {
    /// Count of valid records.
    ValidRecords,
    /// Count of invalid records.
    InvalidRecords,
    /// Completeness percentage (0‑100).
    Completeness,
    /// Timeliness (age of data in seconds).
    Timeliness,
    /// Consistency score (0‑100).
    Consistency,
    /// Accuracy score (0‑100).
    Accuracy,
    /// Custom metric with a name.
    Custom(String),
}

/// Quality metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Timestamp of collection (Unix epoch seconds).
    pub timestamp: u64,
    /// Map from metric name to value.
    pub values: HashMap<String, f64>,
    /// Tags (e.g., data source, agent ID).
    pub tags: HashMap<String, String>,
}

impl QualityMetrics {
    /// Create a new empty metrics snapshot.
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            values: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    /// Add a metric value.
    pub fn add_metric(&mut self, name: impl Into<String>, value: f64) {
        self.values.insert(name.into(), value);
    }

    /// Add a tag.
    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    /// Get a metric value.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Collector for quality metrics.
pub struct MetricCollector {
    registry: Registry,
    valid_records: Counter,
    invalid_records: Counter,
    completeness: Gauge,
    timeliness: Histogram,
}

impl MetricCollector {
    /// Create a new collector with a given namespace.
    pub fn new(namespace: &str) -> Result<Self> {
        let registry = Registry::new();
        let valid_records = Counter::new(
            format!("{}_valid_records_total", namespace),
            "Total number of valid records",
        )?;
        let invalid_records = Counter::new(
            format!("{}_invalid_records_total", namespace),
            "Total number of invalid records",
        )?;
        let completeness = Gauge::new(
            format!("{}_completeness", namespace),
            "Data completeness percentage (0‑100)",
        )?;
        let timeliness = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                format!("{}_timeliness_seconds", namespace),
                "Age of data in seconds",
            )
        )?;

        registry.register(Box::new(valid_records.clone()))?;
        registry.register(Box::new(invalid_records.clone()))?;
        registry.register(Box::new(completeness.clone()))?;
        registry.register(Box::new(timeliness.clone()))?;

        Ok(Self {
            registry,
            valid_records,
            invalid_records,
            completeness,
            timeliness,
        })
    }

    /// Record a valid record.
    pub fn record_valid(&self) {
        self.valid_records.inc();
    }

    /// Record an invalid record.
    pub fn record_invalid(&self) {
        self.invalid_records.inc();
    }

    /// Update completeness percentage.
    pub fn set_completeness(&self, value: f64) {
        self.completeness.set(value);
    }

    /// Observe timeliness (age in seconds).
    pub fn observe_timeliness(&self, age_seconds: f64) {
        self.timeliness.observe(age_seconds);
    }

    /// Export metrics as Prometheus text format.
    pub fn export(&self) -> Result<String> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&self.registry.gather(), &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_metrics() {
        let mut metrics = QualityMetrics::new();
        metrics.add_metric("completeness", 95.5);
        metrics.add_tag("source", "sensor1");
        assert_eq!(metrics.get("completeness"), Some(95.5));
    }

    #[test]
    fn test_metric_collector() -> Result<()> {
        let collector = MetricCollector::new("test")?;
        collector.record_valid();
        collector.record_invalid();
        collector.set_completeness(80.0);
        collector.observe_timeliness(2.5);
        let exported = collector.export()?;
        assert!(exported.contains("test_valid_records_total"));
        Ok(())
    }
}