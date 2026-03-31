//! Scaling policies for autoscaling.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Scaling decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingDecision {
    /// Scale up (add agents).
    ScaleUp(usize),
    /// Scale down (remove agents).
    ScaleDown(usize),
    /// No change.
    NoChange,
}

/// Scaling policy trait.
#[async_trait::async_trait]
pub trait ScalingPolicy: Send + Sync {
    /// Evaluate metrics and produce a scaling decision.
    async fn evaluate(&self, metrics: &ScalingMetrics) -> ScalingDecision;
}

/// Metrics used for scaling decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingMetrics {
    /// Current number of active agents.
    pub agent_count: usize,
    /// Average CPU usage across agents (0‑1).
    pub avg_cpu_usage: f64,
    /// Average memory usage across agents (0‑1).
    pub avg_memory_usage: f64,
    /// Pending task count.
    pub pending_tasks: usize,
    /// Average task latency in milliseconds.
    pub avg_task_latency_ms: f64,
    /// Network bandwidth usage (bytes/sec).
    pub network_bandwidth: f64,
    /// Timestamp of metrics collection.
    pub timestamp: u64,
}

/// Threshold‑based scaling policy.
#[derive(Debug, Clone)]
pub struct ThresholdPolicy {
    /// CPU usage threshold for scaling up (0‑1).
    pub cpu_up_threshold: f64,
    /// CPU usage threshold for scaling down (0‑1).
    pub cpu_down_threshold: f64,
    /// Memory usage threshold for scaling up (0‑1).
    pub memory_up_threshold: f64,
    /// Memory usage threshold for scaling down (0‑1).
    pub memory_down_threshold: f64,
    /// Pending tasks threshold for scaling up.
    pub pending_tasks_up_threshold: usize,
    /// Minimum agent count.
    pub min_agents: usize,
    /// Maximum agent count.
    pub max_agents: usize,
    /// Scale step (how many agents to add/remove).
    pub scale_step: usize,
}

impl Default for ThresholdPolicy {
    fn default() -> Self {
        Self {
            cpu_up_threshold: 0.8,
            cpu_down_threshold: 0.3,
            memory_up_threshold: 0.8,
            memory_down_threshold: 0.3,
            pending_tasks_up_threshold: 10,
            min_agents: 1,
            max_agents: 20,
            scale_step: 1,
        }
    }
}

#[async_trait::async_trait]
impl ScalingPolicy for ThresholdPolicy {
    async fn evaluate(&self, metrics: &ScalingMetrics) -> ScalingDecision {
        let mut need_up = false;
        let mut need_down = false;

        if metrics.avg_cpu_usage > self.cpu_up_threshold
            || metrics.avg_memory_usage > self.memory_up_threshold
            || metrics.pending_tasks > self.pending_tasks_up_threshold
        {
            need_up = true;
        }

        if metrics.avg_cpu_usage < self.cpu_down_threshold
            && metrics.avg_memory_usage < self.memory_down_threshold
            && metrics.pending_tasks == 0
            && metrics.agent_count > self.min_agents
        {
            need_down = true;
        }

        if need_up && metrics.agent_count < self.max_agents {
            ScalingDecision::ScaleUp(self.scale_step)
        } else if need_down && metrics.agent_count > self.min_agents {
            ScalingDecision::ScaleDown(self.scale_step)
        } else {
            ScalingDecision::NoChange
        }
    }
}

/// Predictive scaling policy using moving average.
#[derive(Debug, Clone)]
pub struct PredictivePolicy {
    /// Window size for moving average.
    pub window_size: usize,
    /// History of metrics.
    history: VecDeque<ScalingMetrics>,
    /// Threshold for trend detection.
    pub trend_threshold: f64,
}

impl PredictivePolicy {
    /// Create a new predictive policy.
    pub fn new(window_size: usize, trend_threshold: f64) -> Self {
        Self {
            window_size,
            history: VecDeque::with_capacity(window_size),
            trend_threshold,
        }
    }

    /// Add metrics to history.
    pub fn add_metrics(&mut self, metrics: ScalingMetrics) {
        if self.history.len() >= self.window_size {
            self.history.pop_front();
        }
        self.history.push_back(metrics);
    }

    /// Compute trend of CPU usage.
    fn cpu_trend(&self) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }
        let first = self.history.front().unwrap().avg_cpu_usage;
        let last = self.history.back().unwrap().avg_cpu_usage;
        Some(last - first)
    }
}

#[async_trait::async_trait]
impl ScalingPolicy for PredictivePolicy {
    async fn evaluate(&self, metrics: &ScalingMetrics) -> ScalingDecision {
        // For simplicity, we just use a threshold policy internally.
        let threshold = ThresholdPolicy::default();
        threshold.evaluate(metrics).await
    }
}

/// Composite policy that combines multiple policies.
pub struct CompositePolicy {
    policies: Vec<Box<dyn ScalingPolicy>>,
    /// How to combine decisions (e.g., "any", "all", "weighted").
    combination: String,
}

impl CompositePolicy {
    /// Create a new composite policy.
    pub fn new(policies: Vec<Box<dyn ScalingPolicy>>, combination: &str) -> Self {
        Self {
            policies,
            combination: combination.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl ScalingPolicy for CompositePolicy {
    async fn evaluate(&self, metrics: &ScalingMetrics) -> ScalingDecision {
        let decisions: Vec<ScalingDecision> = futures::future::join_all(
            self.policies.iter().map(|p| p.evaluate(metrics))
        ).await;

        // Simple "any" combination: if any policy says scale up, scale up.
        // If any says scale down, scale down (but scale up takes precedence).
        let mut scale_up = 0;
        let mut scale_down = 0;
        for decision in decisions {
            match decision {
                ScalingDecision::ScaleUp(n) => scale_up = scale_up.max(n),
                ScalingDecision::ScaleDown(n) => scale_down = scale_down.max(n),
                ScalingDecision::NoChange => {}
            }
        }

        if scale_up > 0 {
            ScalingDecision::ScaleUp(scale_up)
        } else if scale_down > 0 {
            ScalingDecision::ScaleDown(scale_down)
        } else {
            ScalingDecision::NoChange
        }
    }
}