//! Metrics collection utilities for autoscaling.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Detailed metrics for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Agent ID.
    pub agent_id: u64,
    /// CPU usage (0‑1).
    pub cpu_usage: f64,
    /// Memory usage (0‑1).
    pub memory_usage: f64,
    /// Number of active tasks.
    pub active_tasks: usize,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Network bytes sent.
    pub bytes_sent: u64,
    /// Network bytes received.
    pub bytes_received: u64,
}

/// Aggregated metrics across all agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Per‑agent metrics.
    pub per_agent: HashMap<u64, AgentMetrics>,
    /// Timestamp of collection.
    pub timestamp: u64,
}

impl AggregatedMetrics {
    /// Compute average CPU usage.
    pub fn average_cpu(&self) -> f64 {
        if self.per_agent.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.per_agent.values().map(|m| m.cpu_usage).sum();
        sum / self.per_agent.len() as f64
    }

    /// Compute average memory usage.
    pub fn average_memory(&self) -> f64 {
        if self.per_agent.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.per_agent.values().map(|m| m.memory_usage).sum();
        sum / self.per_agent.len() as f64
    }

    /// Total active tasks.
    pub fn total_active_tasks(&self) -> usize {
        self.per_agent.values().map(|m| m.active_tasks).sum()
    }

    /// Total network throughput (bytes/sec).
    pub fn total_throughput(&self, interval_secs: f64) -> f64 {
        if interval_secs <= 0.0 {
            return 0.0;
        }
        let total_bytes: u64 = self
            .per_agent
            .values()
            .map(|m| m.bytes_sent + m.bytes_received)
            .sum();
        total_bytes as f64 / interval_secs
    }
}

/// Metrics collector trait.
#[async_trait::async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Collect metrics from all agents.
    async fn collect(&self) -> Result<AggregatedMetrics, String>;
}

/// Dummy metrics collector for testing.
pub struct DummyMetricsCollector;

#[async_trait::async_trait]
impl MetricsCollector for DummyMetricsCollector {
    async fn collect(&self) -> Result<AggregatedMetrics, String> {
        let mut per_agent = HashMap::new();
        // Simulate three agents.
        for i in 1..=3 {
            per_agent.insert(
                i,
                AgentMetrics {
                    agent_id: i,
                    cpu_usage: 0.3 + (i as f64 * 0.1),
                    memory_usage: 0.4 + (i as f64 * 0.05),
                    active_tasks: i as usize * 2,
                    uptime_secs: 1000 * i,
                    bytes_sent: 5000 * i,
                    bytes_received: 8000 * i,
                },
            );
        }
        Ok(AggregatedMetrics {
            per_agent,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}