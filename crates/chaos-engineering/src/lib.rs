//! Chaos engineering for testing system resilience.
//!
//! Provides:
//! - Chaos experiments
//! - Attack types (latency, error, kill, resource)
//! - Steady state hypothesis
//! - Blast radius control
//! - Automated rollback

pub mod experiment;
pub mod attack;
pub mod hypothesis;
pub mod monitor;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use experiment::*;
pub use attack::*;
pub use hypothesis::*;
pub use monitor::*;

/// Chaos engineering configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosConfig {
    pub enable_chaos: bool,
    pub max_blast_radius: f64,
    pub default_duration_secs: u64,
    pub auto_rollback: bool,
    pub monitoring_interval_secs: u64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            enable_chaos: true,
            max_blast_radius: 0.3,
            default_duration_secs: 300,
            auto_rollback: true,
            monitoring_interval_secs: 5,
        }
    }
}

/// Chaos engineering manager.
pub struct ChaosManager {
    config: ChaosConfig,
    experiments: RwLock<HashMap<String, ChaosExperiment>>,
    active_experiments: RwLock<Vec<String>>,
    metrics: RwLock<ChaosMetrics>,
}

impl ChaosManager {
    /// Create new chaos manager.
    pub fn new(config: ChaosConfig) -> Self {
        Self {
            config,
            experiments: RwLock::new(HashMap::new()),
            active_experiments: RwLock::new(Vec::new()),
            metrics: RwLock::new(ChaosMetrics::default()),
        }
    }

    /// Initialize chaos engineering.
    pub async fn initialize(&self) -> Result<()> {
        info!("Chaos engineering initialized with blast radius: {}", self.config.max_blast_radius);
        Ok(())
    }

    /// Create chaos experiment.
    pub async fn create_experiment(&self, name: &str, experiment: ChaosExperiment) -> Result<()> {
        self.experiments.write().await.insert(name.to_string(), experiment);
        info!("Chaos experiment created: {}", name);
        Ok(())
    }

    /// Start experiment.
    pub async fn start_experiment(&self, name: &str) -> Result<String> {
        let experiments = self.experiments.read().await;
        
        let experiment = experiments.get(name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found: {}", name))?;

        // Validate blast radius
        if experiment.blast_radius > self.config.max_blast_radius {
            return Err(anyhow::anyhow!(
                "Blast radius {} exceeds maximum {}",
                experiment.blast_radius,
                self.config.max_blast_radius
            ));
        }

        // Verify steady state hypothesis
        let hypothesis = experiment.hypothesis.clone();
        if !hypothesis.verify().await? {
            return Err(anyhow::anyhow!("Steady state hypothesis not met"));
        }

        // Start attacks
        for attack in &experiment.attacks {
            attack.execute().await?;
        }

        self.active_experiments.write().await.push(name.to_string());
        
        let run_id = uuid::Uuid::new_v4().to_string();
        info!("Chaos experiment started: {} (run: {})", name, run_id);

        Ok(run_id)
    }

    /// Stop experiment.
    pub async fn stop_experiment(&self, name: &str) -> Result<()> {
        let experiments = self.experiments.read().await;
        
        let experiment = experiments.get(name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found: {}", name))?;

        // Rollback attacks
        for attack in &experiment.attacks {
            attack.rollback().await?;
        }

        self.active_experiments.write().await.retain(|n| n != name);
        info!("Chaos experiment stopped: {}", name);

        Ok(())
    }

    /// Run experiment with automatic rollback.
    pub async fn run_experiment(&self, name: &str) -> Result<ExperimentResult> {
        let experiments = self.experiments.read().await;
        
        let experiment = experiments.get(name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found: {}", name))?;

        let run_id = self.start_experiment(name).await?;
        let start_time = chrono::Utc::now();

        // Monitor during experiment
        let duration = std::time::Duration::from_secs(experiment.duration_secs);
        tokio::time::sleep(duration).await;

        // Check hypothesis
        let hypothesis_valid = experiment.hypothesis.verify().await?;
        
        // Stop experiment (rollback)
        self.stop_experiment(name).await?;

        let end_time = chrono::Utc::now();
        let result = ExperimentResult {
            run_id,
            experiment_name: name.to_string(),
            start_time,
            end_time,
            success: hypothesis_valid,
            hypothesis_valid,
            attacks_executed: experiment.attacks.len() as i32,
        };

        // Update metrics
        self.update_metrics(&result).await;

        Ok(result)
    }

    /// Update metrics.
    async fn update_metrics(&self, result: &ExperimentResult) {
        let mut metrics = self.metrics.write().await;
        metrics.total_experiments += 1;
        
        if result.success {
            metrics.successful_experiments += 1;
        } else {
            metrics.failed_experiments += 1;
        }

        metrics.last_experiment_time = result.end_time;
    }

    /// Get active experiments.
    pub async fn get_active_experiments(&self) -> Vec<String> {
        self.active_experiments.read().await.clone()
    }

    /// Get experiment status.
    pub async fn get_experiment_status(&self, name: &str) -> Result<ExperimentStatus> {
        let experiments = self.experiments.read().await;
        let active = self.active_experiments.read().await;

        let experiment = experiments.get(name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found: {}", name))?;

        let is_active = active.contains(&name.to_string());

        Ok(ExperimentStatus {
            name: name.to_string(),
            active: is_active,
            blast_radius: experiment.blast_radius,
            duration_secs: experiment.duration_secs,
            attacks_count: experiment.attacks.len(),
        })
    }

    /// Get chaos metrics.
    pub async fn get_metrics(&self) -> ChaosMetrics {
        self.metrics.read().await.clone()
    }

    /// Inject failure.
    pub async fn inject_failure(&self, target: &str, failure_type: FailureType) -> Result<()> {
        let attack = match failure_type {
            FailureType::Latency => Attack::latency(target, 1000, 0.5),
            FailureType::Error => Attack::error(target, 0.3),
            FailureType::Kill => Attack::kill(target),
            FailureType::Resource => Attack::resource(target, 0.8),
        };

        attack.execute().await?;
        info!("Failure injected: {:?} on {}", failure_type, target);

        Ok(())
    }

    /// Stop all experiments.
    pub async fn stop_all(&self) -> Result<()> {
        let active = self.active_experiments.read().await.clone();
        
        for name in active {
            self.stop_experiment(&name).await?;
        }

        info!("All chaos experiments stopped");
        Ok(())
    }
}

/// Chaos experiment.
#[derive(Debug, Clone)]
pub struct ChaosExperiment {
    pub name: String,
    pub description: String,
    pub hypothesis: SteadyStateHypothesis,
    pub attacks: Vec<Attack>,
    pub blast_radius: f64,
    pub duration_secs: u64,
    pub rollback_enabled: bool,
}

impl ChaosExperiment {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            hypothesis: SteadyStateHypothesis::default(),
            attacks: Vec::new(),
            blast_radius: 0.1,
            duration_secs: 300,
            rollback_enabled: true,
        }
    }

    pub fn with_hypothesis(mut self, hypothesis: SteadyStateHypothesis) -> Self {
        self.hypothesis = hypothesis;
        self
    }

    pub fn with_attack(mut self, attack: Attack) -> Self {
        self.attacks.push(attack);
        self
    }

    pub fn with_blast_radius(mut self, radius: f64) -> Self {
        self.blast_radius = radius;
        self
    }

    pub fn with_duration(mut self, duration_secs: u64) -> Self {
        self.duration_secs = duration_secs;
        self
    }
}

/// Experiment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub run_id: String,
    pub experiment_name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub hypothesis_valid: bool,
    pub attacks_executed: i32,
}

/// Experiment status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStatus {
    pub name: String,
    pub active: bool,
    pub blast_radius: f64,
    pub duration_secs: u64,
    pub attacks_count: usize,
}

/// Chaos metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChaosMetrics {
    pub total_experiments: i64,
    pub successful_experiments: i64,
    pub failed_experiments: i64,
    pub last_experiment_time: chrono::DateTime<chrono::Utc>,
}

/// Failure types for quick injection.
#[derive(Debug, Clone)]
pub enum FailureType {
    Latency,
    Error,
    Kill,
    Resource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chaos_manager() {
        let config = ChaosConfig::default();
        let manager = ChaosManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Create experiment
        let experiment = ChaosExperiment::new("test-experiment", "Test")
            .with_blast_radius(0.1)
            .with_duration(60);

        manager.create_experiment("test", experiment).await.unwrap();

        // Get status
        let status = manager.get_experiment_status("test").await.unwrap();
        assert!(!status.active);
        assert_eq!(status.blast_radius, 0.1);

        // Get metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_experiments, 0);
    }
}
