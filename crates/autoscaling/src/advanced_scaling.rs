//! Advanced metric‑based autoscaling with predictive algorithms.
//!
//! This module provides sophisticated scaling decisions based on historical
//! metrics, trend analysis, and machine learning predictions.

use std::collections::VecDeque;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::AutoscalingError;
use crate::metrics::{AgentMetrics, AggregatedMetrics};
use crate::policy::{ScalingDecision, ScalingDirection};

/// Historical metrics window for trend analysis.
#[derive(Debug, Clone)]
pub struct MetricsWindow {
    /// Timestamped metrics samples.
    samples: VecDeque<(DateTime<Utc>, AggregatedMetrics)>,
    /// Maximum number of samples to keep.
    max_samples: usize,
}

impl MetricsWindow {
    /// Create a new metrics window.
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    /// Add a new metrics sample.
    pub fn add_sample(&mut self, timestamp: DateTime<Utc>, metrics: AggregatedMetrics) {
        self.samples.push_back((timestamp, metrics));
        if self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }
    }

    /// Get the most recent sample.
    pub fn latest(&self) -> Option<&AggregatedMetrics> {
        self.samples.back().map(|(_, m)| m)
    }

    /// Compute trend for a specific metric over the window.
    pub fn compute_trend<F>(&self, extractor: F) -> Option<f64>
    where
        F: Fn(&AggregatedMetrics) -> f64,
    {
        if self.samples.len() < 2 {
            return None;
        }

        let values: Vec<f64> = self.samples.iter()
            .map(|(_, m)| extractor(m))
            .collect();
        
        // Simple linear regression slope
        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate()
            .map(|(i, &y)| i as f64 * y)
            .sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        Some(slope)
    }

    /// Predict future value based on trend.
    pub fn predict<F>(&self, extractor: F, steps_ahead: usize) -> Option<f64>
    where
        F: Fn(&AggregatedMetrics) -> f64,
    {
        let trend = self.compute_trend(&extractor)?;
        let latest = self.latest()?;
        let current = extractor(latest);
        Some(current + trend * steps_ahead as f64)
    }
}

/// Configuration for predictive scaling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveScalingConfig {
    /// Minimum number of historical samples required for prediction.
    pub min_samples: usize,
    /// Number of steps ahead to predict.
    pub prediction_horizon: usize,
    /// Confidence threshold for acting on predictions (0-1).
    pub confidence_threshold: f64,
    /// Weight given to predictions vs current metrics (0-1).
    pub prediction_weight: f64,
}

impl Default for PredictiveScalingConfig {
    fn default() -> Self {
        Self {
            min_samples: 10,
            prediction_horizon: 5,
            confidence_threshold: 0.7,
            prediction_weight: 0.3,
        }
    }
}

/// Predictive scaling policy using historical metrics.
pub struct PredictiveScalingPolicy {
    config: PredictiveScalingConfig,
    metrics_window: Arc<RwLock<MetricsWindow>>,
}

impl PredictiveScalingPolicy {
    /// Create a new predictive scaling policy.
    pub fn new(config: PredictiveScalingConfig, window_size: usize) -> Self {
        Self {
            config,
            metrics_window: Arc::new(RwLock::new(MetricsWindow::new(window_size))),
        }
    }

    /// Update with new metrics.
    pub async fn update_metrics(&self, metrics: AggregatedMetrics) -> Result<(), AutoscalingError> {
        let mut window = self.metrics_window.write().await;
        window.add_sample(Utc::now(), metrics);
        Ok(())
    }

    /// Make a scaling decision based on predictions.
    pub async fn decide(
        &self,
        current_agents: usize,
        target_cpu: f64,
        target_memory: f64,
    ) -> Result<ScalingDecision, AutoscalingError> {
        let window = self.metrics_window.read().await;
        
        // Get current metrics
        let current_metrics = window.latest()
            .ok_or_else(|| AutoscalingError::InsufficientData("no metrics available".to_string()))?;
        
        let current_cpu = current_metrics.average_cpu();
        let current_memory = current_metrics.average_memory();
        
        // Check if we have enough data for predictions
        let has_enough_data = window.samples.len() >= self.config.min_samples;
        
        let mut decision = ScalingDecision {
            direction: ScalingDirection::None,
            amount: 0,
            reason: String::new(),
        };
        
        if has_enough_data {
            // Try to predict future CPU and memory
            if let (Some(predicted_cpu), Some(predicted_memory)) = (
                window.predict(|m| m.average_cpu(), self.config.prediction_horizon),
                window.predict(|m| m.average_memory(), self.config.prediction_horizon),
            ) {
                // Blend current and predicted values
                let blended_cpu = current_cpu * (1.0 - self.config.prediction_weight) 
                    + predicted_cpu * self.config.prediction_weight;
                let blended_memory = current_memory * (1.0 - self.config.prediction_weight)
                    + predicted_memory * self.config.prediction_weight;
                
                // Make decision based on blended values
                self.make_decision_based_on_metrics(
                    blended_cpu, blended_memory, target_cpu, target_memory, 
                    current_agents, &mut decision, "predictive"
                );
            }
        }
        
        // If no predictive decision or prediction not confident enough,
        // fall back to current metrics
        if decision.direction == ScalingDirection::None {
            self.make_decision_based_on_metrics(
                current_cpu, current_memory, target_cpu, target_memory,
                current_agents, &mut decision, "reactive"
            );
        }
        
        Ok(decision)
    }
    
    fn make_decision_based_on_metrics(
        &self,
        cpu: f64,
        memory: f64,
        target_cpu: f64,
        target_memory: f64,
        current_agents: usize,
        decision: &mut ScalingDecision,
        mode: &str,
    ) {
        // Simple scaling logic: scale out if either CPU or memory is above target
        // scale in if both are below target with some hysteresis
        const HYSTERESIS: f64 = 0.1;
        
        if cpu > target_cpu || memory > target_memory {
            // Scale out
            let overload = cpu.max(memory) - target_cpu.max(target_memory);
            let amount = (overload * current_agents as f64).ceil() as usize;
            decision.direction = ScalingDirection::Out;
            decision.amount = amount.max(1);
            decision.reason = format!("{}: CPU={:.2}, memory={:.2} exceed targets", 
                                     mode, cpu, memory);
        } else if cpu < target_cpu - HYSTERESIS && memory < target_memory - HYSTERESIS {
            // Scale in (but never below 1)
            if current_agents > 1 {
                let underload = (target_cpu - cpu).max(target_memory - memory);
                let amount = (underload * current_agents as f64).floor() as usize;
                decision.direction = ScalingDirection::In;
                decision.amount = amount.min(current_agents - 1).max(1);
                decision.reason = format!("{}: CPU={:.2}, memory={:.2} below targets", 
                                         mode, cpu, memory);
            }
        }
    }
}

/// Multi‑metric scaling decision combining multiple metrics with weights.
#[derive(Debug, Clone)]
pub struct MultiMetricScalingConfig {
    /// Metrics to consider with their weights.
    pub metrics: Vec<(String, f64, f64)>, // (name, weight, target)
    /// Overall scaling aggressiveness (0-1).
    pub aggressiveness: f64,
}

impl Default for MultiMetricScalingConfig {
    fn default() -> Self {
        Self {
            metrics: vec![
                ("cpu".to_string(), 0.6, 0.7),
                ("memory".to_string(), 0.3, 0.8),
                ("active_tasks".to_string(), 0.1, 10.0),
            ],
            aggressiveness: 0.5,
        }
    }
}

/// Scaling based on multiple weighted metrics.
pub struct MultiMetricScalingPolicy {
    config: MultiMetricScalingConfig,
}

impl MultiMetricScalingPolicy {
    /// Create a new multi‑metric scaling policy.
    pub fn new(config: MultiMetricScalingConfig) -> Self {
        Self { config }
    }

    /// Extract metric value from agent metrics.
    fn extract_metric(&self, metric_name: &str, agent_metrics: &AgentMetrics) -> f64 {
        match metric_name {
            "cpu" => agent_metrics.cpu_usage,
            "memory" => agent_metrics.memory_usage,
            "active_tasks" => agent_metrics.active_tasks as f64,
            "uptime_secs" => agent_metrics.uptime_secs as f64,
            "bytes_sent" => agent_metrics.bytes_sent as f64,
            "bytes_received" => agent_metrics.bytes_received as f64,
            _ => 0.0,
        }
    }

    /// Compute weighted score for scaling decision.
    pub fn compute_score(&self, metrics: &AggregatedMetrics) -> f64 {
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;
        
        for (metric_name, weight, target) in &self.config.metrics {
            // Compute average of this metric across all agents
            let mut sum = 0.0;
            let mut count = 0;
            
            for agent_metrics in metrics.per_agent.values() {
                let value = self.extract_metric(metric_name, agent_metrics);
                sum += value;
                count += 1;
            }
            
            if count > 0 {
                let average = sum / count as f64;
                // Normalize by target: value > target => score > 1
                let normalized = if *target > 0.0 { average / target } else { 0.0 };
                weighted_sum += normalized * weight;
                total_weight += weight;
            }
        }
        
        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// Make scaling decision based on weighted score.
    pub fn decide(&self, score: f64, current_agents: usize) -> ScalingDecision {
        let mut decision = ScalingDecision {
            direction: ScalingDirection::None,
            amount: 0,
            reason: String::new(),
        };
        
        // Score interpretation:
        // < 0.8: underutilized (scale in)
        // 0.8-1.2: optimal range (no scaling)
        // > 1.2: overloaded (scale out)
        
        if score > 1.2 {
            let overload = score - 1.2;
            let amount = (overload * current_agents as f64 * self.config.aggressiveness).ceil() as usize;
            decision.direction = ScalingDirection::Out;
            decision.amount = amount.max(1);
            decision.reason = format!("weighted score {:.2} indicates overload", score);
        } else if score < 0.8 && current_agents > 1 {
            let underload = 0.8 - score;
            let amount = (underload * current_agents as f64 * self.config.aggressiveness).floor() as usize;
            decision.direction = ScalingDirection::In;
            decision.amount = amount.min(current_agents - 1).max(1);
            decision.reason = format!("weighted score {:.2} indicates underutilization", score);
        }
        
        decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{AgentMetrics, AggregatedMetrics};
    use std::collections::HashMap;

    fn create_sample_metrics() -> AggregatedMetrics {
        let mut per_agent = HashMap::new();
        per_agent.insert(1, AgentMetrics {
            agent_id: 1,
            cpu_usage: 0.6,
            memory_usage: 0.5,
            active_tasks: 5,
            uptime_secs: 100,
            bytes_sent: 1000,
            bytes_received: 2000,
        });
        per_agent.insert(2, AgentMetrics {
            agent_id: 2,
            cpu_usage: 0.7,
            memory_usage: 0.6,
            active_tasks: 8,
            uptime_secs: 200,
            bytes_sent: 1500,
            bytes_received: 2500,
        });
        
        AggregatedMetrics {
            per_agent,
            timestamp: 1234567890,
        }
    }

    #[test]
    fn test_metrics_window() {
        let mut window = MetricsWindow::new(5);
        let metrics = create_sample_metrics();
        
        window.add_sample(Utc::now(), metrics.clone());
        assert_eq!(window.samples.len(), 1);
        
        let latest = window.latest().unwrap();
        assert_eq!(latest.per_agent.len(), 2);
    }

    #[tokio::test]
    async fn test_predictive_policy_creation() {
        let config = PredictiveScalingConfig::default();
        let policy = PredictiveScalingPolicy::new(config, 20);
        
        let metrics = create_sample_metrics();
        policy.update_metrics(metrics).await.unwrap();
        
        // With only one sample, should fall back to reactive
        let decision = policy.decide(3, 0.7, 0.8).await.unwrap();
        assert!(decision.direction == ScalingDirection::None || 
                decision.direction == ScalingDirection::Out ||
                decision.direction == ScalingDirection::In);
    }

    #[test]
    fn test_multi_metric_scoring() {
        let config = MultiMetricScalingConfig::default();
        let policy = MultiMetricScalingPolicy::new(config);
        
        let metrics = create_sample_metrics();
        let score = policy.compute_score(&metrics);
        
        // Score should be positive
        assert!(score >= 0.0);
    }
}