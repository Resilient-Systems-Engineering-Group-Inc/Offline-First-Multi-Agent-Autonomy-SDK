//! Load metrics collection and analysis.

use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};
use crate::error::{LoadBalancingError, Result};

/// Comprehensive load metrics for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoad {
    /// CPU utilization (0.0 to 1.0).
    pub cpu_utilization: f64,
    /// Memory utilization (0.0 to 1.0).
    pub memory_utilization: f64,
    /// Network I/O utilization (0.0 to 1.0).
    pub network_utilization: f64,
    /// Disk I/O utilization (0.0 to 1.0).
    pub disk_utilization: f64,
    /// Number of active tasks.
    pub active_tasks: usize,
    /// Queue length (tasks waiting).
    pub queue_length: usize,
    /// Response time in milliseconds.
    pub response_time_ms: f64,
    /// Error rate (0.0 to 1.0).
    pub error_rate: f64,
    /// Custom metrics.
    pub custom_metrics: HashMap<String, f64>,
    /// Timestamp when metrics were collected.
    pub timestamp: SystemTime,
}

impl AgentLoad {
    /// Create a new AgentLoad with default values.
    pub fn new(cpu_utilization: f64) -> Self {
        Self {
            cpu_utilization,
            memory_utilization: 0.0,
            network_utilization: 0.0,
            disk_utilization: 0.0,
            active_tasks: 0,
            queue_length: 0,
            response_time_ms: 0.0,
            error_rate: 0.0,
            custom_metrics: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    /// Create a comprehensive AgentLoad.
    pub fn comprehensive(
        cpu_utilization: f64,
        memory_utilization: f64,
        active_tasks: usize,
        queue_length: usize,
        response_time_ms: f64,
    ) -> Self {
        Self {
            cpu_utilization,
            memory_utilization,
            network_utilization: 0.0,
            disk_utilization: 0.0,
            active_tasks,
            queue_length,
            response_time_ms,
            error_rate: 0.0,
            custom_metrics: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }

    /// Calculate total load score (0.0 to 1.0).
    pub fn total_load(&self) -> f64 {
        let weights = LoadWeights::default();
        weights.calculate_total(self)
    }

    /// Check if agent is overloaded.
    pub fn is_overloaded(&self, threshold: f64) -> bool {
        self.total_load() > threshold
    }

    /// Check if agent is healthy (error rate below threshold).
    pub fn is_healthy(&self, error_threshold: f64) -> bool {
        self.error_rate < error_threshold
    }

    /// Add a custom metric.
    pub fn add_custom_metric(&mut self, key: &str, value: f64) {
        self.custom_metrics.insert(key.to_string(), value);
    }

    /// Get a custom metric.
    pub fn get_custom_metric(&self, key: &str) -> Option<f64> {
        self.custom_metrics.get(key).copied()
    }
}

impl Default for AgentLoad {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Weights for calculating total load score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadWeights {
    /// Weight for CPU utilization.
    pub cpu_weight: f64,
    /// Weight for memory utilization.
    pub memory_weight: f64,
    /// Weight for network utilization.
    pub network_weight: f64,
    /// Weight for disk utilization.
    pub disk_weight: f64,
    /// Weight for active tasks.
    pub tasks_weight: f64,
    /// Weight for queue length.
    pub queue_weight: f64,
    /// Weight for response time.
    pub response_weight: f64,
    /// Weight for error rate.
    pub error_weight: f64,
}

impl LoadWeights {
    /// Create new load weights.
    pub fn new(
        cpu_weight: f64,
        memory_weight: f64,
        network_weight: f64,
        disk_weight: f64,
        tasks_weight: f64,
        queue_weight: f64,
        response_weight: f64,
        error_weight: f64,
    ) -> Self {
        Self {
            cpu_weight,
            memory_weight,
            network_weight,
            disk_weight,
            tasks_weight,
            queue_weight,
            response_weight,
            error_weight,
        }
    }

    /// Calculate total load score.
    pub fn calculate_total(&self, load: &AgentLoad) -> f64 {
        let mut total = 0.0;
        
        // Normalize active tasks and queue length (assuming max 100 tasks/queue)
        let normalized_tasks = (load.active_tasks as f64).min(100.0) / 100.0;
        let normalized_queue = (load.queue_length as f64).min(100.0) / 100.0;
        
        // Normalize response time (assuming max 1000ms)
        let normalized_response = (load.response_time_ms.min(1000.0)) / 1000.0;
        
        total += load.cpu_utilization * self.cpu_weight;
        total += load.memory_utilization * self.memory_weight;
        total += load.network_utilization * self.network_weight;
        total += load.disk_utilization * self.disk_weight;
        total += normalized_tasks * self.tasks_weight;
        total += normalized_queue * self.queue_weight;
        total += normalized_response * self.response_weight;
        total += load.error_rate * self.error_weight;
        
        // Cap at 1.0
        total.min(1.0)
    }
}

impl Default for LoadWeights {
    fn default() -> Self {
        // Default weights emphasizing CPU and memory
        Self::new(0.3, 0.2, 0.1, 0.1, 0.1, 0.1, 0.05, 0.05)
    }
}

/// Load metrics for multiple agents.
#[derive(Debug, Clone, Default)]
pub struct LoadMetrics {
    /// Map from agent ID to load.
    pub agent_loads: HashMap<String, AgentLoad>,
    /// Global statistics.
    pub statistics: LoadStatistics,
    /// Timestamp of last update.
    pub last_update: SystemTime,
}

impl LoadMetrics {
    /// Create new empty load metrics.
    pub fn new() -> Self {
        Self {
            agent_loads: HashMap::new(),
            statistics: LoadStatistics::default(),
            last_update: SystemTime::now(),
        }
    }

    /// Update metrics for an agent.
    pub fn update_agent(&mut self, agent_id: &str, load: AgentLoad) {
        self.agent_loads.insert(agent_id.to_string(), load);
        self.update_statistics();
        self.last_update = SystemTime::now();
    }

    /// Remove an agent's metrics.
    pub fn remove_agent(&mut self, agent_id: &str) {
        self.agent_loads.remove(agent_id);
        self.update_statistics();
    }

    /// Get agent load.
    pub fn get_agent_load(&self, agent_id: &str) -> Option<&AgentLoad> {
        self.agent_loads.get(agent_id)
    }

    /// Find the least loaded agent.
    pub fn find_least_loaded(&self) -> Option<(&String, &AgentLoad)> {
        self.agent_loads
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.total_load().partial_cmp(&b.total_load()).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Find overloaded agents.
    pub fn find_overloaded(&self, threshold: f64) -> Vec<(&String, &AgentLoad)> {
        self.agent_loads
            .iter()
            .filter(|(_, load)| load.total_load() > threshold)
            .collect()
    }

    /// Update statistics.
    fn update_statistics(&mut self) {
        let loads: Vec<f64> = self.agent_loads.values().map(|l| l.total_load()).collect();
        
        if loads.is_empty() {
            self.statistics = LoadStatistics::default();
            return;
        }
        
        let count = loads.len();
        let sum: f64 = loads.iter().sum();
        let avg = sum / count as f64;
        
        let variance: f64 = loads.iter()
            .map(|&x| (x - avg).powi(2))
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();
        
        let min = loads.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = loads.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        self.statistics = LoadStatistics {
            agent_count: count,
            average_load: avg,
            min_load: min,
            max_load: max,
            std_dev_load: std_dev,
            imbalance_score: (max - min).max(0.0),
        };
    }
}

/// Global load statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadStatistics {
    /// Number of agents.
    pub agent_count: usize,
    /// Average load across all agents.
    pub average_load: f64,
    /// Minimum load.
    pub min_load: f64,
    /// Maximum load.
    pub max_load: f64,
    /// Standard deviation of loads.
    pub std_dev_load: f64,
    /// Load imbalance score (max - min).
    pub imbalance_score: f64,
}

impl Default for LoadStatistics {
    fn default() -> Self {
        Self {
            agent_count: 0,
            average_load: 0.0,
            min_load: 0.0,
            max_load: 0.0,
            std_dev_load: 0.0,
            imbalance_score: 0.0,
        }
    }
}

/// Collector for gathering load metrics from agents.
pub struct LoadMetricsCollector {
    /// Metrics storage.
    metrics: LoadMetrics,
    /// Collection interval.
    collection_interval: Duration,
    /// Whether collection is active.
    active: bool,
}

impl LoadMetricsCollector {
    /// Create a new metrics collector.
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            metrics: LoadMetrics::new(),
            collection_interval,
            active: false,
        }
    }

    /// Start collecting metrics.
    pub async fn start(&mut self) -> Result<()> {
        self.active = true;
        // In a real implementation, this would spawn a background task
        // that periodically collects metrics from agents
        Ok(())
    }

    /// Stop collecting metrics.
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Manually update metrics for an agent.
    pub fn update(&mut self, agent_id: &str, load: AgentLoad) {
        self.metrics.update_agent(agent_id, load);
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> &LoadMetrics {
        &self.metrics
    }

    /// Get mutable metrics.
    pub fn get_metrics_mut(&mut self) -> &mut LoadMetrics {
        &mut self.metrics
    }

    /// Check if collector is active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}