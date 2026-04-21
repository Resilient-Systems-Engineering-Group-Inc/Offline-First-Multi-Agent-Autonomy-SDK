//! Metrics collection and Prometheus integration.

use prometheus::{Encoder, Gauge, Histogram, IntCounter, Registry, TextEncoder};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Prometheus metrics registry.
pub struct MetricsCollector {
    registry: Registry,
    
    // Counters
    pub tasks_completed_total: IntCounter,
    pub tasks_failed_total: IntCounter,
    pub tasks_pending_total: IntCounter,
    pub messages_sent_total: IntCounter,
    pub messages_received_total: IntCounter,
    pub consensus_round_total: IntCounter,
    pub consensus_success_total: IntCounter,
    pub consensus_timeout_total: IntCounter,
    
    // Gauges
    pub active_agents: Gauge,
    pub connected_peers: Gauge,
    pub crdt_keys_count: Gauge,
    pub workflow_instances: Gauge,
    pub battery_level: Gauge,
    pub cpu_usage: Gauge,
    pub memory_usage: Gauge,
    
    // Histograms
    pub message_latency: Histogram,
    pub consensus_time: Histogram,
    pub task_duration: Histogram,
    pub sync_duration: Histogram,
    
    // Internal state
    pub total_agents: usize,
    pub active_agents_count: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub pending_tasks: usize,
    pub network_latency_ms: f64,
    pub message_rate: f64,
    pub consensus_rounds: u64,
    pub avg_consensus_time_ms: f64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        // Create counters
        let tasks_completed_total = IntCounter::new(
            "sdk_tasks_completed_total",
            "Total number of completed tasks",
        ).unwrap();
        
        let tasks_failed_total = IntCounter::new(
            "sdk_tasks_failed_total",
            "Total number of failed tasks",
        ).unwrap();
        
        let tasks_pending_total = IntCounter::new(
            "sdk_tasks_pending_total",
            "Total number of pending tasks",
        ).unwrap();
        
        let messages_sent_total = IntCounter::new(
            "sdk_messages_sent_total",
            "Total messages sent",
        ).unwrap();
        
        let messages_received_total = IntCounter::new(
            "sdk_messages_received_total",
            "Total messages received",
        ).unwrap();
        
        let consensus_round_total = IntCounter::new(
            "sdk_consensus_rounds_total",
            "Total consensus rounds",
        ).unwrap();
        
        let consensus_success_total = IntCounter::new(
            "sdk_consensus_success_total",
            "Successful consensus rounds",
        ).unwrap();
        
        let consensus_timeout_total = IntCounter::new(
            "sdk_consensus_timeout_total",
            "Consensus timeouts",
        ).unwrap();

        // Create gauges
        let active_agents = Gauge::new(
            "sdk_active_agents",
            "Number of active agents",
        ).unwrap();
        
        let connected_peers = Gauge::new(
            "sdk_connected_peers",
            "Number of connected peers",
        ).unwrap();
        
        let crdt_keys_count = Gauge::new(
            "sdk_crdt_keys_count",
            "Number of keys in CRDT map",
        ).unwrap();
        
        let workflow_instances = Gauge::new(
            "sdk_workflow_instances",
            "Number of active workflow instances",
        ).unwrap();
        
        let battery_level = Gauge::new(
            "sdk_agent_battery_level",
            "Agent battery level",
        ).unwrap();
        
        let cpu_usage = Gauge::new(
            "sdk_cpu_usage_percent",
            "CPU usage percentage",
        ).unwrap();
        
        let memory_usage = Gauge::new(
            "sdk_memory_usage_percent",
            "Memory usage percentage",
        ).unwrap();

        // Create histograms
        let message_latency = Histogram::new(
            "sdk_message_latency_ms",
            "Message latency in milliseconds",
            &[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0],
        ).unwrap();
        
        let consensus_time = Histogram::new(
            "sdk_consensus_time_ms",
            "Consensus round time in milliseconds",
            &[5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0],
        ).unwrap();
        
        let task_duration = Histogram::new(
            "sdk_task_duration_secs",
            "Task duration in seconds",
            &[1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0],
        ).unwrap();
        
        let sync_duration = Histogram::new(
            "sdk_sync_duration_ms",
            "State sync duration in milliseconds",
            &[1.0, 5.0, 10.0, 25.0, 50.0, 100.0],
        ).unwrap();

        // Register all metrics
        registry.register(Box::new(tasks_completed_total.clone())).unwrap();
        registry.register(Box::new(tasks_failed_total.clone())).unwrap();
        registry.register(Box::new(tasks_pending_total.clone())).unwrap();
        registry.register(Box::new(messages_sent_total.clone())).unwrap();
        registry.register(Box::new(messages_received_total.clone())).unwrap();
        registry.register(Box::new(consensus_round_total.clone())).unwrap();
        registry.register(Box::new(consensus_success_total.clone())).unwrap();
        registry.register(Box::new(consensus_timeout_total.clone())).unwrap();
        registry.register(Box::new(active_agents.clone())).unwrap();
        registry.register(Box::new(connected_peers.clone())).unwrap();
        registry.register(Box::new(crdt_keys_count.clone())).unwrap();
        registry.register(Box::new(workflow_instances.clone())).unwrap();
        registry.register(Box::new(battery_level.clone())).unwrap();
        registry.register(Box::new(cpu_usage.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();
        registry.register(Box::new(message_latency.clone())).unwrap();
        registry.register(Box::new(consensus_time.clone())).unwrap();
        registry.register(Box::new(task_duration.clone())).unwrap();
        registry.register(Box::new(sync_duration.clone())).unwrap();

        Self {
            registry,
            tasks_completed_total,
            tasks_failed_total,
            tasks_pending_total,
            messages_sent_total,
            messages_received_total,
            consensus_round_total,
            consensus_success_total,
            consensus_timeout_total,
            active_agents,
            connected_peers,
            crdt_keys_count,
            workflow_instances,
            battery_level,
            cpu_usage,
            memory_usage,
            message_latency,
            consensus_time,
            task_duration,
            sync_duration,
            total_agents: 0,
            active_agents_count: 0,
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            pending_tasks: 0,
            network_latency_ms: 0.0,
            message_rate: 0.0,
            consensus_rounds: 0,
            avg_consensus_time_ms: 0.0,
        }
    }

    /// Generate Prometheus metrics text.
    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)?;
        
        Ok(String::from_utf8(buffer).unwrap_or_default())
    }

    // Task metrics
    pub fn task_completed(&self) {
        self.tasks_completed_total.inc();
        self.completed_tasks += 1;
        self.pending_tasks = self.pending_tasks.saturating_sub(1);
    }

    pub fn task_failed(&self) {
        self.tasks_failed_total.inc();
        self.failed_tasks += 1;
    }

    pub fn task_pending(&self) {
        self.tasks_pending_total.inc();
        self.pending_tasks += 1;
    }

    pub fn observe_task_duration(&self, duration: Duration) {
        self.task_duration.observe(duration.as_secs_f64());
    }

    // Message metrics
    pub fn message_sent(&self) {
        self.messages_sent_total.inc();
    }

    pub fn message_received(&self) {
        self.messages_received_total.inc();
    }

    pub fn observe_message_latency(&self, latency_ms: f64) {
        self.message_latency.observe(latency_ms);
    }

    // Consensus metrics
    pub fn consensus_round(&self) {
        self.consensus_round_total.inc();
        self.consensus_rounds += 1;
    }

    pub fn consensus_success(&self, time_ms: f64) {
        self.consensus_success_total.inc();
        self.consensus_time.observe(time_ms);
        
        // Update average
        let total_time = self.avg_consensus_time_ms * (self.consensus_rounds as f64 - 1.0) + time_ms;
        self.avg_consensus_time_ms = total_time / self.consensus_rounds as f64;
    }

    pub fn consensus_timeout(&self) {
        self.consensus_timeout_total.inc();
    }

    pub fn observe_consensus_time(&self, time_ms: f64) {
        self.consensus_time.observe(time_ms);
    }

    // Agent metrics
    pub fn set_active_agents(&self, count: usize) {
        self.active_agents.set(count as f64);
        self.active_agents_count = count;
    }

    pub fn set_connected_peers(&self, count: usize) {
        self.connected_peers.set(count as f64);
    }

    pub fn set_battery_level(&self, level: f64) {
        self.battery_level.set(level);
    }

    pub fn set_cpu_usage(&self, usage: f64) {
        self.cpu_usage.set(usage);
    }

    pub fn set_memory_usage(&self, usage: f64) {
        self.memory_usage.set(usage);
    }

    // CRDT metrics
    pub fn set_crdt_keys(&self, count: usize) {
        self.crdt_keys_count.set(count as f64);
    }

    // Workflow metrics
    pub fn set_workflow_instances(&self, count: usize) {
        self.workflow_instances.set(count as f64);
    }

    // Sync metrics
    pub fn observe_sync_duration(&self, duration_ms: f64) {
        self.sync_duration.observe(duration_ms);
    }

    // Update internal state
    pub fn update_network_latency(&mut self, latency_ms: f64) {
        self.network_latency_ms = latency_ms;
    }

    pub fn update_message_rate(&mut self, rate: f64) {
        self.message_rate = rate;
    }

    /// Start metrics collection background task.
    pub fn start_collector(self: Arc<Self>, interval_secs: u64) {
        let collector = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                
                // Collect system metrics
                if let Ok(cpu) = get_cpu_usage() {
                    collector.set_cpu_usage(cpu);
                }
                
                if let Ok(mem) = get_memory_usage() {
                    collector.set_memory_usage(mem);
                }
                
                collector.update_message_rate(
                    collector.messages_sent_total.get() as f64 / interval_secs as f64
                );
            }
        });
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions (would use sysinfo or similar in production)
fn get_cpu_usage() -> Result<f64, ()> {
    // Placeholder - implement with sysinfo crate
    Ok(0.0)
}

fn get_memory_usage() -> Result<f64, ()> {
    // Placeholder - implement with sysinfo crate
    Ok(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collection() {
        let collector = MetricsCollector::new();
        
        collector.task_completed();
        collector.task_completed();
        collector.task_failed();
        
        assert_eq!(collector.completed_tasks, 2);
        assert_eq!(collector.failed_tasks, 1);
        
        let metrics = collector.gather().unwrap();
        assert!(metrics.contains("sdk_tasks_completed_total"));
    }

    #[test]
    fn test_histograms() {
        let collector = MetricsCollector::new();
        
        collector.observe_message_latency(10.5);
        collector.observe_consensus_time(25.0);
        collector.observe_task_duration(Duration::from_secs(30));
        
        let metrics = collector.gather().unwrap();
        assert!(metrics.contains("sdk_message_latency_ms"));
        assert!(metrics.contains("sdk_consensus_time_ms"));
    }
}
