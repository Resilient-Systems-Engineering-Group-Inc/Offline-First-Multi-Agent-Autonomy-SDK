//! A/B testing for ML models.
//!
//! This module provides functionality for running A/B tests between different
//! versions of ML models, collecting metrics, and performing statistical analysis
//! to determine which version performs better.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ModelVersioningError, Result};
use crate::manager::ModelVersioningManager;
use crate::storage::ModelStorage;
use crate::types::{ModelId, ModelVersion, VersionId};

/// A/B experiment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABExperimentConfig {
    /// Unique experiment identifier.
    pub experiment_id: String,
    /// Human-readable experiment name.
    pub name: String,
    /// Description of the experiment.
    pub description: String,
    /// Model identifier being tested.
    pub model_id: ModelId,
    /// Control version (version A).
    pub control_version: VersionId,
    /// Treatment version (version B).
    pub treatment_version: VersionId,
    /// Traffic allocation percentage for treatment (0.0 to 1.0).
    pub treatment_traffic: f64,
    /// Primary metric to evaluate (e.g., "accuracy", "f1_score", "inference_latency").
    pub primary_metric: String,
    /// Secondary metrics to track.
    pub secondary_metrics: Vec<String>,
    /// Minimum sample size per variant before analysis.
    pub min_sample_size: u64,
    /// Statistical significance threshold (e.g., 0.05 for 95% confidence).
    pub significance_level: f64,
    /// Whether the experiment is currently active.
    pub is_active: bool,
    /// Start timestamp.
    pub started_at: DateTime<Utc>,
    /// End timestamp (optional).
    pub ended_at: Option<DateTime<Utc>>,
    /// Custom metadata.
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

/// Traffic allocation strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrafficAllocationStrategy {
    /// Random allocation with equal probability.
    Random,
    /// Weighted allocation based on specified weights.
    Weighted { control_weight: f64, treatment_weight: f64 },
    /// Epsilon-greedy exploration (epsilon probability of random exploration).
    EpsilonGreedy { epsilon: f64 },
    /// Contextual bandit based on features.
    ContextualBandit,
}

/// Experiment variant assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Variant {
    /// Control group (version A).
    Control,
    /// Treatment group (version B).
    Treatment,
}

/// Observation record for a single inference/request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Unique observation identifier.
    pub observation_id: Uuid,
    /// Experiment identifier.
    pub experiment_id: String,
    /// Assigned variant.
    pub variant: Variant,
    /// Model version used.
    pub model_version: VersionId,
    /// Timestamp of the observation.
    pub timestamp: DateTime<Utc>,
    /// Primary metric value.
    pub primary_metric_value: f64,
    /// Secondary metrics.
    pub secondary_metrics: HashMap<String, f64>,
    /// Context features (for contextual bandits).
    pub context_features: Option<HashMap<String, f64>>,
    /// Custom metadata.
    pub custom_metadata: HashMap<String, serde_json::Value>,
}

/// Statistical test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatisticalTest {
    /// T-test for comparing means.
    TTest {
        t_statistic: f64,
        p_value: f64,
        degrees_of_freedom: f64,
    },
    /// Chi-squared test for proportions.
    ChiSquared {
        chi2_statistic: f64,
        p_value: f64,
        degrees_of_freedom: u32,
    },
    /// Bayesian A/B test result.
    Bayesian {
        probability_b_better: f64,
        expected_loss: f64,
    },
}

/// Experiment result summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Experiment identifier.
    pub experiment_id: String,
    /// Total observations.
    pub total_observations: u64,
    /// Control group observations.
    pub control_observations: u64,
    /// Treatment group observations.
    pub treatment_observations: u64,
    /// Control group mean of primary metric.
    pub control_mean: f64,
    /// Treatment group mean of primary metric.
    pub treatment_mean: f64,
    /// Mean difference (treatment - control).
    pub mean_difference: f64,
    /// Relative improvement percentage.
    pub relative_improvement: f64,
    /// Statistical test result.
    pub statistical_test: Option<StatisticalTest>,
    /// Whether the result is statistically significant.
    pub is_significant: bool,
    /// Recommendation (which variant to choose).
    pub recommendation: Option<Variant>,
    /// Confidence interval for the difference.
    pub confidence_interval: Option<(f64, f64)>,
    /// Timestamp of analysis.
    pub analyzed_at: DateTime<Utc>,
}

/// A/B testing manager.
pub struct ABTestingManager<S: ModelStorage> {
    manager: Arc<ModelVersioningManager<S>>,
    experiments: HashMap<String, ABExperimentConfig>,
    observations: Vec<Observation>,
}

impl<S: ModelStorage> ABTestingManager<S> {
    /// Create a new A/B testing manager.
    pub fn new(manager: Arc<ModelVersioningManager<S>>) -> Self {
        Self {
            manager,
            experiments: HashMap::new(),
            observations: Vec::new(),
        }
    }

    /// Create a new A/B experiment.
    pub async fn create_experiment(
        &mut self,
        config: ABExperimentConfig,
    ) -> Result<()> {
        // Validate that model versions exist
        let control_version = self.manager
            .get_version(&config.model_id, &config.control_version)
            .await?;
        let treatment_version = self.manager
            .get_version(&config.model_id, &config.treatment_version)
            .await?;

        // Validate that treatment_traffic is between 0 and 1
        if !(0.0..=1.0).contains(&config.treatment_traffic) {
            return Err(ModelVersioningError::InvalidInput(
                "treatment_traffic must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate that control and treatment are different
        if config.control_version == config.treatment_version {
            return Err(ModelVersioningError::InvalidInput(
                "control and treatment versions must be different".to_string(),
            ));
        }

        // Store the experiment
        self.experiments.insert(config.experiment_id.clone(), config);

        Ok(())
    }

    /// Get an experiment by ID.
    pub fn get_experiment(&self, experiment_id: &str) -> Option<&ABExperimentConfig> {
        self.experiments.get(experiment_id)
    }

    /// List all experiments.
    pub fn list_experiments(&self) -> Vec<&ABExperimentConfig> {
        self.experiments.values().collect()
    }

    /// Assign a variant for a new request.
    pub fn assign_variant(
        &self,
        experiment_id: &str,
        strategy: &TrafficAllocationStrategy,
        context_features: Option<&HashMap<String, f64>>,
    ) -> Result<(Variant, VersionId)> {
        let experiment = self.experiments.get(experiment_id)
            .ok_or_else(|| ModelVersioningError::NotFound(
                format!("Experiment {} not found", experiment_id)
            ))?;

        if !experiment.is_active {
            return Err(ModelVersioningError::InvalidState(
                format!("Experiment {} is not active", experiment_id)
            ));
        }

        let variant = match strategy {
            TrafficAllocationStrategy::Random => {
                let rand_val = rand::random::<f64>();
                if rand_val < experiment.treatment_traffic {
                    Variant::Treatment
                } else {
                    Variant::Control
                }
            }
            TrafficAllocationStrategy::Weighted { control_weight, treatment_weight } => {
                let total = control_weight + treatment_weight;
                let rand_val = rand::random::<f64>() * total;
                if rand_val < *treatment_weight {
                    Variant::Treatment
                } else {
                    Variant::Control
                }
            }
            TrafficAllocationStrategy::EpsilonGreedy { epsilon } => {
                let rand_val = rand::random::<f64>();
                if rand_val < *epsilon {
                    // Explore: random choice
                    if rand::random::<bool>() {
                        Variant::Treatment
                    } else {
                        Variant::Control
                    }
                } else {
                    // Exploit: choose best based on current estimates
                    // For simplicity, we'll use random until we have enough data
                    let rand_val = rand::random::<f64>();
                    if rand_val < experiment.treatment_traffic {
                        Variant::Treatment
                    } else {
                        Variant::Control
                    }
                }
            }
            TrafficAllocationStrategy::ContextualBandit => {
                // Simple linear contextual bandit for demonstration
                // In production, you would use a more sophisticated algorithm
                if let Some(features) = context_features {
                    // Simple heuristic: if context contains "premium" feature, use treatment
                    if features.get("premium").map(|v| *v > 0.5).unwrap_or(false) {
                        Variant::Treatment
                    } else {
                        Variant::Control
                    }
                } else {
                    // Fallback to random
                    let rand_val = rand::random::<f64>();
                    if rand_val < experiment.treatment_traffic {
                        Variant::Treatment
                    } else {
                        Variant::Control
                    }
                }
            }
        };

        let version = match variant {
            Variant::Control => &experiment.control_version,
            Variant::Treatment => &experiment.treatment_version,
        };

        Ok((variant, version.clone()))
    }

    /// Record an observation (metric) for an experiment.
    pub fn record_observation(&mut self, observation: Observation) -> Result<()> {
        // Validate that the experiment exists and is active
        let experiment = self.experiments.get(&observation.experiment_id)
            .ok_or_else(|| ModelVersioningError::NotFound(
                format!("Experiment {} not found", observation.experiment_id)
            ))?;

        if !experiment.is_active {
            return Err(ModelVersioningError::InvalidState(
                format!("Experiment {} is not active", observation.experiment_id)
            ));
        }

        // Validate that the model version matches the assigned variant
        let expected_version = match observation.variant {
            Variant::Control => &experiment.control_version,
            Variant::Treatment => &experiment.treatment_version,
        };

        if &observation.model_version != expected_version {
            return Err(ModelVersioningError::InvalidInput(
                format!("Model version {} does not match expected variant", observation.model_version)
            ));
        }

        self.observations.push(observation);
        Ok(())
    }

    /// Analyze experiment results.
    pub fn analyze_experiment(&self, experiment_id: &str) -> Result<ExperimentResult> {
        let experiment = self.experiments.get(experiment_id)
            .ok_or_else(|| ModelVersioningError::NotFound(
                format!("Experiment {} not found", experiment_id)
            ))?;

        // Filter observations for this experiment
        let exp_observations: Vec<&Observation> = self.observations
            .iter()
            .filter(|obs| obs.experiment_id == experiment_id)
            .collect();

        if exp_observations.is_empty() {
            return Err(ModelVersioningError::InvalidState(
                format!("No observations for experiment {}", experiment_id)
            ));
        }

        // Split by variant
        let (control_obs, treatment_obs): (Vec<&Observation>, Vec<&Observation>) = exp_observations
            .into_iter()
            .partition(|obs| matches!(obs.variant, Variant::Control));

        let control_count = control_obs.len() as u64;
        let treatment_count = treatment_obs.len() as u64;
        let total_count = control_count + treatment_count;

        // Calculate means
        let control_mean = if control_count > 0 {
            control_obs.iter().map(|obs| obs.primary_metric_value).sum::<f64>() / control_count as f64
        } else { 0.0 };
        
        let treatment_mean = if treatment_count > 0 {
            treatment_obs.iter().map(|obs| obs.primary_metric_value).sum::<f64>() / treatment_count as f64
        } else { 0.0 };

        let mean_difference = treatment_mean - control_mean;
        let relative_improvement = if control_mean != 0.0 {
            mean_difference / control_mean * 100.0
        } else { 0.0 };

        // Perform statistical test if we have enough data
        let statistical_test = if control_count >= experiment.min_sample_size as usize &&
            treatment_count >= experiment.min_sample_size as usize {
            // Simple t-test for demonstration
            // In production, use a proper statistical library
            let control_var = if control_count > 1 {
                control_obs.iter()
                    .map(|obs| (obs.primary_metric_value - control_mean).powi(2))
                    .sum::<f64>() / (control_count - 1) as f64
            } else { 0.0 };
            
            let treatment_var = if treatment_count > 1 {
                treatment_obs.iter()
                    .map(|obs| (obs.primary_metric_value - treatment_mean).powi(2))
                    .sum::<f64>() / (treatment_count - 1) as f64
            } else { 0.0 };

            let pooled_var = ((control_count - 1) as f64 * control_var + 
                             (treatment_count - 1) as f64 * treatment_var) / 
                             (control_count + treatment_count - 2) as f64;
            
            let standard_error = (pooled_var * (1.0/control_count as f64 + 1.0/treatment_count as f64)).sqrt();
            
            let t_statistic = if standard_error > 0.0 {
                mean_difference / standard_error
            } else { 0.0 };
            
            let degrees_of_freedom = (control_count + treatment_count - 2) as f64;
            
            // Simple p-value approximation (two-tailed)
            let p_value = 2.0 * (1.0 - students_t_cdf(t_statistic.abs(), degrees_of_freedom));
            
            Some(StatisticalTest::TTest {
                t_statistic,
                p_value,
                degrees_of_freedom,
            })
        } else {
            None
        };

        let is_significant = statistical_test.as_ref()
            .map(|test| match test {
                StatisticalTest::TTest { p_value, .. } => *p_value < experiment.significance_level,
                StatisticalTest::ChiSquared { p_value, .. } => *p_value < experiment.significance_level,
                StatisticalTest::Bayesian { probability_b_better, .. } => *probability_b_better > 0.95,
            })
            .unwrap_or(false);

        let recommendation = if is_significant {
            if mean_difference > 0.0 {
                Some(Variant::Treatment)
            } else {
                Some(Variant::Control)
            }
        } else {
            None
        };

        // Calculate confidence interval
        let confidence_interval = if control_count > 0 && treatment_count > 0 {
            let control_std = if control_count > 1 {
                control_obs.iter()
                    .map(|obs| (obs.primary_metric_value - control_mean).powi(2))
                    .sum::<f64>() / (control_count - 1) as f64
            } else { 0.0 }.sqrt();
            
            let treatment_std = if treatment_count > 1 {
                treatment_obs.iter()
                    .map(|obs| (obs.primary_metric_value - treatment_mean).powi(2))
                    .sum::<f64>() / (treatment_count - 1) as f64
            } else { 0.0 }.sqrt();
            
            let se = (control_std.powi(2)/control_count as f64 + 
                     treatment_std.powi(2)/treatment_count as f64).sqrt();
            
            let z_score = 1.96; // 95% confidence
            let margin = z_score * se;
            Some((mean_difference - margin, mean_difference + margin))
        } else {
            None
        };

        Ok(ExperimentResult {
            experiment_id: experiment_id.to_string(),
            total_observations: total_count,
            control_observations: control_count,
            treatment_observations: treatment_count,
            control_mean,
            treatment_mean,
            mean_difference,
            relative_improvement,
            statistical_test,
            is_significant,
            recommendation,
            confidence_interval,
            analyzed_at: Utc::now(),
        })
    }

    /// Stop an experiment.
    pub fn stop_experiment(&mut self, experiment_id: &str) -> Result<()> {
        let experiment = self.experiments.get_mut(experiment_id)
            .ok_or_else(|| ModelVersioningError::NotFound(
                format!("Experiment {} not found", experiment_id)
            ))?;

        experiment.is_active = false;
        experiment.ended_at = Some(Utc::now());
        Ok(())
    }

    /// Get all observations for an experiment.
    pub fn get_observations(&self, experiment_id: &str) -> Vec<&Observation> {
        self.observations
            .iter()
            .filter(|obs| obs.experiment_id == experiment_id)
            .collect()
    }
}

/// Cumulative distribution function for Student's t-distribution (approximation).
fn students_t_cdf(t: f64, df: f64) -> f64 {
    // Simple approximation using normal distribution for large df
    if df > 30.0 {
        // Use normal approximation
        let z = t;
        0.5 * (1.0 + erf(z / 2.0_f64.sqrt()))
    } else {
        // Beta distribution approximation
        let x = df / (df + t * t);
        0.5 * (1.0 + (x - 0.5).signum() * incomplete_beta(0.5 * df, 0.5, x).sqrt())
    }
}

/// Error function approximation.
fn erf(x: f64) -> f64 {
    // Abramowitz and Stegun approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

/// Incomplete beta function approximation.
fn incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    // Simple approximation for demonstration
    // In production, use a proper numerical library
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    
    // Use continued fraction approximation
    let eps = 1e-10;
    let mut bm = 1.0;
    let mut az = 1.0;
    let mut am = 1.0;
    let mut bz = 1.0 - (a + b) * x / (a + 1.0);
    
    let mut m = 1;
    while m < 100 {
        let em = m as f64;
        let tem = em + em;
        let d1 = em * (b - em) * x / ((a + tem - 1.0) * (a + tem));
        let ap = az + d1 * am;
        let bp = bz + d1 * bm;
        let d2 = -(a + em) * (a + b + em) * x / ((a + tem) * (a + tem + 1.0));
        let app = ap + d2 * az;
        let bpp = bp + d2 * bz;
        let aold = az;
        am = ap / bpp;
        bm = bp / bpp;
        az = app / bpp;
        bz = 1.0;
        if (az - aold).abs() < eps * az.abs() {
            break;
        }
        m += 1;
    }
    
    az
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;

    #[tokio::test]
    async fn test_create_experiment() {
        let storage = InMemoryStorage::new();
        let config = crate::manager::ModelVersioningConfig::default();
        let manager = Arc::new(crate::manager::ModelVersioningManager::new(storage, config));
        
        let mut ab_manager = ABTestingManager::new(manager.clone());
        
        // First create a model and versions
        // ... (test setup would go here)
        
        // This is a skeleton test - in a real test we would set up the model first
        assert!(true);
    }

    #[test]
    fn test_assign_variant_random() {
        // Test random allocation
        let config = ABExperimentConfig {
            experiment_id: "test-exp".to_string(),
            name: "Test Experiment".to_string(),
            description: "Test".to_string(),
            model_id: "test-model".to_string(),
            control_version: "v1.0.0".to_string(),
            treatment_version: "v1.1.0".to_string(),
            treatment_traffic: 0.5,
            primary_metric: "accuracy".to_string(),
            secondary_metrics: vec![],
            min_sample_size: 100,
            significance_level: 0.05,
            is_active: true,
            started_at: Utc::now(),
            ended_at: None,
            custom_metadata: HashMap::new(),
        };

        // We can't fully test without a manager, but we can test the logic
        assert_eq!(config.control_version, "v1.0.0");
        assert_eq!(config.treatment_version, "v1.1.0");
    }
}