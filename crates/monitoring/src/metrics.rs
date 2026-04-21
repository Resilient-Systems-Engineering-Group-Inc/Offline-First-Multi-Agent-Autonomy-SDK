//! Prometheus metrics collection.

use prometheus::{Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Registry};
use std::sync::Arc;

/// SDK Metrics collector.
pub struct SdkMetrics {
    // Task metrics
    pub tasks_total: IntCounter,
    pub tasks_completed: IntCounter,
    pub tasks_failed: IntCounter,
    pub task_duration_seconds: Histogram,

    // Agent metrics
    pub agents_total: IntGauge,
    pub agents_active: IntGauge,
    pub agents_offline: IntGauge,

    // Network metrics
    pub messages_sent: IntCounter,
    pub messages_received: IntCounter,
    pub network_latency_seconds: Histogram,

    // Performance metrics
    pub request_duration_seconds: Histogram,
    pub active_connections: IntGauge,
    pub cpu_usage_percent: Gauge,
    pub memory_usage_bytes: Gauge,
}

impl SdkMetrics {
    /// Create new metrics collector.
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        // Task metrics
        let tasks_total = IntCounter::new("sdk_tasks_total", "Total number of tasks")?;
        let tasks_completed = IntCounter::new("sdk_tasks_completed_total", "Completed tasks")?;
        let tasks_failed = IntCounter::new("sdk_tasks_failed_total", "Failed tasks")?;
        
        let task_duration_opts = HistogramOpts::new(
            "sdk_task_duration_seconds",
            "Task execution duration"
        ).buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]);
        let task_duration_seconds = Histogram::with_opts(task_duration_opts)?;

        // Agent metrics
        let agents_total = IntGauge::new("sdk_agents_total", "Total agents")?;
        let agents_active = IntGauge::new("sdk_agents_active", "Active agents")?;
        let agents_offline = IntGauge::new("sdk_agents_offline", "Offline agents")?;

        // Network metrics
        let messages_sent = IntCounter::new("sdk_messages_sent_total", "Messages sent")?;
        let messages_received = IntCounter::new("sdk_messages_received_total", "Messages received")?;
        
        let latency_opts = HistogramOpts::new(
            "sdk_network_latency_seconds",
            "Network latency"
        ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]);
        let network_latency_seconds = Histogram::with_opts(latency_opts)?;

        // Performance metrics
        let request_opts = HistogramOpts::new(
            "sdk_request_duration_seconds",
            "HTTP request duration"
        ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]);
        let request_duration_seconds = Histogram::with_opts(request_opts)?;
        
        let active_connections = IntGauge::new("sdk_active_connections", "Active connections")?;
        let cpu_usage_percent = Gauge::new("sdk_cpu_usage_percent", "CPU usage")?;
        let memory_usage_bytes = Gauge::new("sdk_memory_usage_bytes", "Memory usage")?;

        // Register all metrics
        registry.register(Box::new(tasks_total.clone()))?;
        registry.register(Box::new(tasks_completed.clone()))?;
        registry.register(Box::new(tasks_failed.clone()))?;
        registry.register(Box::new(task_duration_seconds.clone()))?;
        registry.register(Box::new(agents_total.clone()))?;
        registry.register(Box::new(agents_active.clone()))?;
        registry.register(Box::new(agents_offline.clone()))?;
        registry.register(Box::new(messages_sent.clone()))?;
        registry.register(Box::new(messages_received.clone()))?;
        registry.register(Box::new(network_latency_seconds.clone()))?;
        registry.register(Box::new(request_duration_seconds.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;

        Ok(Self {
            tasks_total,
            tasks_completed,
            tasks_failed,
            task_duration_seconds,
            agents_total,
            agents_active,
            agents_offline,
            messages_sent,
            messages_received,
            network_latency_seconds,
            request_duration_seconds,
            active_connections,
            cpu_usage_percent,
            memory_usage_bytes,
        })
    }

    /// Increment task counter.
    pub fn inc_tasks_total(&self) {
        self.tasks_total.inc();
    }

    /// Increment completed tasks.
    pub fn inc_tasks_completed(&self) {
        self.tasks_completed.inc();
    }

    /// Increment failed tasks.
    pub fn inc_tasks_failed(&self) {
        self.tasks_failed.inc();
    }

    /// Record task duration.
    pub fn observe_task_duration(&self, duration: f64) {
        self.task_duration_seconds.observe(duration);
    }

    /// Set agent counts.
    pub fn set_agents(&self, total: i64, active: i64, offline: i64) {
        self.agents_total.set(total);
        self.agents_active.set(active);
        self.agents_offline.set(offline);
    }

    /// Increment message counters.
    pub fn inc_messages_sent(&self) {
        self.messages_sent.inc();
    }

    pub fn inc_messages_received(&self) {
        self.messages_received.inc();
    }

    /// Record network latency.
    pub fn observe_latency(&self, latency: f64) {
        self.network_latency_seconds.observe(latency);
    }

    /// Record request duration.
    pub fn observe_request_duration(&self, duration: f64) {
        self.request_duration_seconds.observe(duration);
    }

    /// Set active connections.
    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count);
    }

    /// Set CPU usage.
    pub fn set_cpu_usage(&self, percent: f64) {
        self.cpu_usage_percent.set(percent);
    }

    /// Set memory usage.
    pub fn set_memory_usage(&self, bytes: f64) {
        self.memory_usage_bytes.set(bytes);
    }
}

/// Guard for timing operations.
pub struct TimingGuard<'a> {
    histogram: &'a Histogram,
    start: std::time::Instant,
}

impl<'a> TimingGuard<'a> {
    pub fn new(histogram: &'a Histogram) -> Self {
        Self {
            histogram,
            start: std::time::Instant::now(),
        }
    }
}

impl<'a> Drop for TimingGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

/// Helper trait for timing operations.
pub trait MeasureDuration {
    fn observe<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R;
}

impl MeasureDuration for Histogram {
    fn observe<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed().as_secs_f64();
        self.observe(duration);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_metrics() {
        let registry = Registry::new();
        let metrics = SdkMetrics::new(&registry).unwrap();

        // Test counters
        metrics.inc_tasks_total();
        metrics.inc_tasks_completed();
        
        // Test gauges
        metrics.set_agents(10, 8, 2);
        metrics.set_cpu_usage(45.5);
        
        // Test histograms
        metrics.observe_task_duration(1.5);
        metrics.observe_latency(0.05);

        // Test timing guard
        {
            let _guard = TimingGuard::new(&metrics.task_duration_seconds);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
