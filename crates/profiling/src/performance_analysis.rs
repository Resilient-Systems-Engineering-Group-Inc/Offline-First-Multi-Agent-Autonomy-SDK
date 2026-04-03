//! Advanced performance analysis tools for distributed systems.
//!
//! This module provides utilities to measure, aggregate, and analyze
//! performance metrics across a swarm of agents.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use metrics::{Histogram, Key, Recorder, Unit};
use metrics_exporter_prometheus::PrometheusRecorder;
use common::types::AgentId;
use crate::metrics as profiling_metrics;

/// Tracks latency samples for a given operation.
#[derive(Clone, Debug)]
pub struct LatencyTracker {
    samples: VecDeque<Duration>,
    max_samples: usize,
}

impl LatencyTracker {
    /// Create a new tracker with a sliding window of `max_samples`.
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    /// Record a latency sample.
    pub fn record(&mut self, latency: Duration) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(latency);
    }

    /// Compute the mean latency.
    pub fn mean(&self) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        let sum: Duration = self.samples.iter().sum();
        Some(sum / self.samples.len() as u32)
    }

    /// Compute a percentile latency (0.0‑1.0).
    pub fn percentile(&self, p: f64) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted: Vec<Duration> = self.samples.iter().cloned().collect();
        sorted.sort();
        let idx = (p * (sorted.len() - 1) as f64).round() as usize;
        Some(sorted[idx])
    }

    /// Reset all samples.
    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

/// Measures throughput (operations per second).
#[derive(Clone, Debug)]
pub struct ThroughputMeter {
    count: u64,
    window_start: Instant,
    window_duration: Duration,
    history: VecDeque<(Instant, u64)>,
}

impl ThroughputMeter {
    /// Create a new meter with a sliding window of `window_duration`.
    pub fn new(window_duration: Duration) -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            window_duration,
            history: VecDeque::new(),
        }
    }

    /// Increment the operation count.
    pub fn increment(&mut self) {
        self.count += 1;
        let now = Instant::now();
        if now.duration_since(self.window_start) >= self.window_duration {
            self.history.push_back((self.window_start, self.count));
            self.window_start = now;
            self.count = 0;
            // Keep only recent history (last 10 windows)
            if self.history.len() > 10 {
                self.history.pop_front();
            }
        }
    }

    /// Get current throughput (ops/sec) averaged over the window.
    pub fn current_throughput(&self) -> f64 {
        let elapsed = self.window_start.elapsed();
        if elapsed.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.count as f64 / elapsed.as_secs_f64()
    }

    /// Get average throughput over the last N windows.
    pub fn average_throughput(&self, last_windows: usize) -> f64 {
        let windows = self.history.iter().rev().take(last_windows);
        let total: u64 = windows.map(|(_, count)| count).sum();
        let total_duration = self.window_duration * last_windows as u32;
        total as f64 / total_duration.as_secs_f64()
    }
}

/// Correlates resource usage with performance metrics.
pub struct ResourceCorrelator {
    cpu_samples: VecDeque<f32>,
    latency_samples: VecDeque<Duration>,
    max_samples: usize,
}

impl ResourceCorrelator {
    pub fn new(max_samples: usize) -> Self {
        Self {
            cpu_samples: VecDeque::with_capacity(max_samples),
            latency_samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    /// Record a pair (cpu_usage_percent, latency).
    pub fn record(&mut self, cpu: f32, latency: Duration) {
        if self.cpu_samples.len() >= self.max_samples {
            self.cpu_samples.pop_front();
            self.latency_samples.pop_front();
        }
        self.cpu_samples.push_back(cpu);
        self.latency_samples.push_back(latency);
    }

    /// Compute Pearson correlation coefficient between CPU and latency.
    pub fn correlation(&self) -> Option<f64> {
        let n = self.cpu_samples.len();
        if n < 2 {
            return None;
        }
        let cpu_mean: f64 = self.cpu_samples.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
        let lat_mean: f64 = self.latency_samples.iter().map(|d| d.as_secs_f64()).sum::<f64>() / n as f64;

        let mut cov = 0.0;
        let mut var_cpu = 0.0;
        let mut var_lat = 0.0;
        for (cpu, lat) in self.cpu_samples.iter().zip(self.latency_samples.iter()) {
            let cpu_diff = *cpu as f64 - cpu_mean;
            let lat_diff = lat.as_secs_f64() - lat_mean;
            cov += cpu_diff * lat_diff;
            var_cpu += cpu_diff * cpu_diff;
            var_lat += lat_diff * lat_diff;
        }
        if var_cpu == 0.0 || var_lat == 0.0 {
            return None;
        }
        Some(cov / (var_cpu.sqrt() * var_lat.sqrt()))
    }
}

/// Central performance analysis manager.
pub struct PerformanceAnalyzer {
    latency_trackers: RwLock<HashMap<String, LatencyTracker>>,
    throughput_meters: RwLock<HashMap<String, ThroughputMeter>>,
    correlators: RwLock<HashMap<String, ResourceCorrelator>>,
    alert_thresholds: HashMap<String, AlertThreshold>,
}

/// Threshold for triggering performance alerts.
#[derive(Clone, Debug)]
pub struct AlertThreshold {
    pub metric: String,
    pub condition: Condition,
    pub severity: Severity,
}

#[derive(Clone, Debug)]
pub enum Condition {
    LatencyAbove(Duration),
    ThroughputBelow(f64),
    CorrelationAbove(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Warning,
    Critical,
}

#[derive(Clone, Debug)]
pub struct Alert {
    pub metric: String,
    pub condition: Condition,
    pub severity: Severity,
    pub timestamp: Instant,
    pub value: String,
}

impl PerformanceAnalyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            latency_trackers: RwLock::new(HashMap::new()),
            throughput_meters: RwLock::new(HashMap::new()),
            correlators: RwLock::new(HashMap::new()),
            alert_thresholds: HashMap::new(),
        }
    }

    /// Record a latency sample for a given operation.
    pub async fn record_latency(&self, operation: &str, latency: Duration) {
        let mut trackers = self.latency_trackers.write().await;
        let tracker = trackers
            .entry(operation.to_string())
            .or_insert_with(|| LatencyTracker::new(1000));
        tracker.record(latency);
        // Also update global metrics
        profiling_metrics::record_histogram(
            "operation_latency_seconds",
            latency.as_secs_f64(),
            &[("operation", operation)],
        );
    }

    /// Increment throughput for a given operation.
    pub async fn increment_throughput(&self, operation: &str) {
        let mut meters = self.throughput_meters.write().await;
        let meter = meters
            .entry(operation.to_string())
            .or_insert_with(|| ThroughputMeter::new(Duration::from_secs(10)));
        meter.increment();
    }

    /// Record resource correlation.
    pub async fn record_correlation(&self, correlation_id: &str, cpu: f32, latency: Duration) {
        let mut correlators = self.correlators.write().await;
        let correlator = correlators
            .entry(correlation_id.to_string())
            .or_insert_with(|| ResourceCorrelator::new(1000));
        correlator.record(cpu, latency);
    }

    /// Get latency statistics for an operation.
    pub async fn latency_stats(&self, operation: &str) -> Option<LatencyStats> {
        let trackers = self.latency_trackers.read().await;
        trackers.get(operation).map(|tracker| LatencyStats {
            mean: tracker.mean(),
            p50: tracker.percentile(0.5),
            p95: tracker.percentile(0.95),
            p99: tracker.percentile(0.99),
            sample_count: tracker.samples.len(),
        })
    }

    /// Check all thresholds and return triggered alerts.
    pub async fn check_alerts(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let now = Instant::now();
        // Check latency thresholds
        let trackers = self.latency_trackers.read().await;
        for (metric, threshold) in &self.alert_thresholds {
            match &threshold.condition {
                Condition::LatencyAbove(limit) => {
                    if let Some(tracker) = trackers.get(metric) {
                        if let Some(p95) = tracker.percentile(0.95) {
                            if p95 > *limit {
                                alerts.push(Alert {
                                    metric: metric.clone(),
                                    condition: threshold.condition.clone(),
                                    severity: threshold.severity.clone(),
                                    timestamp: now,
                                    value: format!("{:.3?}", p95),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        alerts
    }

    /// Export all metrics as JSON for offline analysis.
    pub async fn export_json(&self) -> serde_json::Value {
        let trackers = self.latency_trackers.read().await;
        let meters = self.throughput_meters.read().await;
        let mut data = serde_json::json!({});
        for (op, tracker) in trackers.iter() {
            data[op] = serde_json::json!({
                "mean_ms": tracker.mean().map(|d| d.as_millis()),
                "p95_ms": tracker.percentile(0.95).map(|d| d.as_millis()),
                "samples": tracker.samples.len(),
            });
        }
        for (op, meter) in meters.iter() {
            data[op]["throughput_ops_per_sec"] = serde_json::json!(meter.current_throughput());
        }
        data
    }
}

/// Statistics for latency.
#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub mean: Option<Duration>,
    pub p50: Option<Duration>,
    pub p95: Option<Duration>,
    pub p99: Option<Duration>,
    pub sample_count: usize,
}

/// Initialize performance analysis and start background aggregation.
pub async fn init_performance_analysis() -> Arc<PerformanceAnalyzer> {
    let analyzer = Arc::new(PerformanceAnalyzer::new());
    // Spawn a background task to periodically log stats
    let analyzer_clone = analyzer.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let alerts = analyzer_clone.check_alerts().await;
            for alert in alerts {
                tracing::warn!(
                    "Performance alert: {} {} {:?}",
                    alert.metric,
                    alert.value,
                    alert.severity
                );
            }
        }
    });
    analyzer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_tracker() {
        let mut tracker = LatencyTracker::new(10);
        tracker.record(Duration::from_millis(100));
        tracker.record(Duration::from_millis(200));
        assert_eq!(tracker.mean(), Some(Duration::from_millis(150)));
        assert!(tracker.percentile(0.5).unwrap() >= Duration::from_millis(100));
    }

    #[test]
    fn test_throughput_meter() {
        let mut meter = ThroughputMeter::new(Duration::from_secs(1));
        meter.increment();
        meter.increment();
        // Should be >0
        assert!(meter.current_throughput() > 0.0);
    }

    #[tokio::test]
    async fn test_analyzer() {
        let analyzer = PerformanceAnalyzer::new();
        analyzer.record_latency("test_op", Duration::from_millis(50)).await;
        let stats = analyzer.latency_stats("test_op").await;
        assert!(stats.is_some());
    }
}