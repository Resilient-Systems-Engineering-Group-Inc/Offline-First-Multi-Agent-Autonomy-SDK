//! Advanced differential privacy techniques for federated learning.
//!
//! This module provides state‑of‑the‑art differential privacy algorithms
//! specifically designed for distributed machine learning scenarios:
//! - Renyi Differential Privacy (RDP) accounting
//! - Gaussian Mechanism with tight bounds
//! - Moments Accountant for composition
//! - Privacy Amplification by Sampling
//! - Distributed DP with secure aggregation
//! - Adaptive clipping and noise scaling

use rand::{Rng, distributions::{Normal, Uniform}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

/// Renyi Differential Privacy (RDP) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdpConfig {
    /// Alpha values for Renyi divergence computation.
    pub alphas: Vec<f64>,
    /// Maximum alpha to consider.
    pub max_alpha: f64,
    /// Order of Renyi divergence.
    pub orders: Vec<f64>,
}

impl Default for RdpConfig {
    fn default() -> Self {
        Self {
            alphas: vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0],
            max_alpha: 64.0,
            orders: vec![1.5, 2.0, 4.0, 8.0, 16.0, 32.0],
        }
    }
}

/// Advanced differential privacy with Renyi accounting.
pub struct AdvancedDifferentialPrivacy {
    /// Base privacy configuration.
    config: crate::privacy::DifferentialPrivacyConfig,
    /// RDP configuration.
    rdp_config: RdpConfig,
    /// Privacy budget spent so far.
    spent_budget: (f64, f64),
    /// Privacy accountant.
    accountant: MomentsAccountant,
}

impl AdvancedDifferentialPrivacy {
    /// Create a new advanced DP engine.
    pub fn new(
        config: crate::privacy::DifferentialPrivacyConfig,
        rdp_config: Option<RdpConfig>,
    ) -> Self {
        Self {
            config: config.clone(),
            rdp_config: rdp_config.unwrap_or_default(),
            spent_budget: (0.0, 0.0),
            accountant: MomentsAccountant::new(config),
        }
    }

    /// Apply Gaussian mechanism with optimal noise scaling.
    pub fn gaussian_mechanism(&self, vector: &[f64], sensitivity: f64) -> Vec<f64> {
        let sigma = sensitivity * (2.0 * (1.25 / self.config.delta).ln()).sqrt() / self.config.epsilon;
        
        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, sigma).unwrap();
        
        vector
            .iter()
            .map(|&x| x + rng.sample(normal))
            .collect()
    }

    /// Apply the Laplace mechanism.
    pub fn laplace_mechanism(&self, vector: &[f64], sensitivity: f64) -> Vec<f64> {
        let scale = sensitivity / self.config.epsilon;
        let mut rng = rand::thread_rng();
        let uniform = Uniform::new(-0.5, 0.5);
        
        vector
            .iter()
            .map(|&x| {
                let u = rng.sample(uniform);
                x + scale * (u.signum() * (1.0 - 2.0 * u.abs()).ln())
            })
            .collect()
    }

    /// Adaptive clipping based on gradient norm distribution.
    pub fn adaptive_clip(&self, vectors: &[Vec<f64>], percentile: f64) -> f64 {
        if vectors.is_empty() {
            return self.config.clip_norm;
        }
        
        let norms: Vec<f64> = vectors
            .iter()
            .map(|v| v.iter().map(|x| x * x).sum::<f64>().sqrt())
            .collect();
        
        let mut sorted = norms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((percentile / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[index].max(1e-6)
    }

    /// Apply DP-SGD (Differentially Private Stochastic Gradient Descent).
    pub fn dp_sgd(
        &self,
        gradients: &[Vec<f64>],
        learning_rate: f64,
        batch_size: usize,
        total_samples: usize,
    ) -> (Vec<f64>, (f64, f64)) {
        if gradients.is_empty() {
            return (vec![], (0.0, 0.0));
        }

        let dimension = gradients[0].len();
        
        // 1. Clip each gradient
        let clipped_gradients: Vec<Vec<f64>> = gradients
            .iter()
            .map(|g| self.clip_by_norm(g))
            .collect();
        
        // 2. Compute average gradient
        let mut avg_gradient = vec![0.0; dimension];
        for grad in &clipped_gradients {
            for (i, &val) in grad.iter().enumerate() {
                avg_gradient[i] += val;
            }
        }
        let count = gradients.len() as f64;
        for val in &mut avg_gradient {
            *val /= count;
        }
        
        // 3. Add noise
        let sensitivity = self.config.clip_norm / batch_size as f64;
        let noisy_gradient = self.gaussian_mechanism(&avg_gradient, sensitivity);
        
        // 4. Scale by learning rate
        let update: Vec<f64> = noisy_gradient
            .iter()
            .map(|&x| -learning_rate * x)
            .collect();
        
        // 5. Account privacy cost
        let privacy_cost = self.accountant.compute_privacy_cost(
            batch_size,
            total_samples,
            gradients.len(),
        );
        
        (update, privacy_cost)
    }

    /// Compute Renyi differential privacy bounds.
    pub fn rdp_bound(&self, alpha: f64, sigma: f64) -> f64 {
        // RDP for Gaussian mechanism: alpha/(2*sigma^2)
        alpha / (2.0 * sigma * sigma)
    }

    /// Convert RDP to (epsilon, delta)-DP.
    pub fn rdp_to_dp(&self, rdp_epsilon: f64, delta: f64) -> f64 {
        rdp_epsilon + (1.0 / (self.rdp_config.max_alpha - 1.0)) * 
            ((1.0 - 1.0 / self.rdp_config.max_alpha).ln() - delta.ln())
    }

    /// Get current privacy budget.
    pub fn get_spent_budget(&self) -> (f64, f64) {
        self.spent_budget
    }

    /// Clip vector by L2 norm (inherited from base DP).
    fn clip_by_norm(&self, vector: &[f64]) -> Vec<f64> {
        let norm: f64 = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > self.config.clip_norm {
            let scale = self.config.clip_norm / norm;
            vector.iter().map(|&x| x * scale).collect()
        } else {
            vector.to_vec()
        }
    }
}

/// Moments accountant for tracking privacy budget across multiple iterations.
pub struct MomentsAccountant {
    /// Privacy parameters.
    config: crate::privacy::DifferentialPrivacyConfig,
    /// Record of moments.
    moments: HashMap<f64, f64>,
    /// Sampling rate.
    sampling_rate: f64,
    /// Number of steps.
    steps: usize,
}

impl MomentsAccountant {
    /// Create a new moments accountant.
    pub fn new(config: crate::privacy::DifferentialPrivacyConfig) -> Self {
        Self {
            config,
            moments: HashMap::new(),
            sampling_rate: 0.0,
            steps: 0,
        }
    }

    /// Update accountant with a new iteration.
    pub fn update(&mut self, batch_size: usize, total_samples: usize, iterations: usize) {
        self.sampling_rate = batch_size as f64 / total_samples as f64;
        self.steps = iterations;
        
        // Compute moments for Gaussian mechanism
        let sigma = self.config.sigma;
        for &alpha in &[1.5, 2.0, 4.0, 8.0, 16.0, 32.0] {
            let moment = alpha * (alpha - 1.0) / (2.0 * sigma * sigma);
            self.moments.insert(alpha, moment * self.steps as f64);
        }
    }

    /// Compute privacy cost.
    pub fn compute_privacy_cost(
        &self,
        batch_size: usize,
        total_samples: usize,
        iterations: usize,
    ) -> (f64, f64) {
        let sampling_rate = batch_size as f64 / total_samples as f64;
        
        // Use moments accountant formula
        let mut min_epsilon = f64::INFINITY;
        
        for (&alpha, &moment) in &self.moments {
            let epsilon = moment / (alpha - 1.0) + 
                (alpha / (2.0 * (alpha - 1.0))) * 
                (1.0 + (sampling_rate * (alpha - 1.0)).exp_m1()).ln();
            
            if epsilon < min_epsilon {
                min_epsilon = epsilon;
            }
        }
        
        let delta = self.config.delta;
        (min_epsilon, delta)
    }

    /// Get epsilon for a target delta.
    pub fn get_epsilon(&self, target_delta: f64) -> f64 {
        let mut best_epsilon = f64::INFINITY;
        
        for (&alpha, &moment) in &self.moments {
            let epsilon = moment / (alpha - 1.0) + 
                (1.0 / (alpha - 1.0)) * 
                ((alpha / (2.0 * target_delta)).ln());
            
            if epsilon < best_epsilon {
                best_epsilon = epsilon;
            }
        }
        
        best_epsilon
    }
}

/// Privacy amplification by sampling.
pub struct PrivacyAmplification {
    /// Sampling rate.
    sampling_rate: f64,
    /// Amplification factor.
    amplification_factor: f64,
}

impl PrivacyAmplification {
    /// Create a new privacy amplification calculator.
    pub fn new(sampling_rate: f64) -> Self {
        Self {
            sampling_rate,
            amplification_factor: 1.0 / sampling_rate,
        }
    }

    /// Amplify privacy guarantee using sampling.
    pub fn amplify(&self, epsilon: f64, delta: f64) -> (f64, f64) {
        let amplified_epsilon = (1.0 - self.sampling_rate) * epsilon + 
            self.sampling_rate * (epsilon.exp() - 1.0);
        
        let amplified_delta = self.sampling_rate * delta;
        
        (amplified_epsilon, amplified_delta)
    }

    /// Compute optimal sampling rate for target privacy.
    pub fn optimal_sampling_rate(&self, target_epsilon: f64, target_delta: f64) -> f64 {
        // Simplified formula for Gaussian mechanism
        let sigma = (2.0 * (1.25 / target_delta).ln()).sqrt() / target_epsilon;
        (PI * sigma * sigma).sqrt().min(1.0)
    }
}

/// Distributed differential privacy with secure aggregation.
pub struct DistributedDifferentialPrivacy {
    /// Base DP engine.
    dp_engine: Arc<AdvancedDifferentialPrivacy>,
    /// Number of participants.
    participants: usize,
    /// Threshold for secure aggregation.
    threshold: usize,
    /// Privacy amplification enabled.
    amplification_enabled: bool,
}

impl DistributedDifferentialPrivacy {
    /// Create a new distributed DP engine.
    pub fn new(
        dp_engine: Arc<AdvancedDifferentialPrivacy>,
        participants: usize,
        threshold: usize,
    ) -> Self {
        Self {
            dp_engine,
            participants,
            threshold,
            amplification_enabled: true,
        }
    }

    /// Apply distributed DP to model updates from multiple clients.
    pub fn apply_distributed_dp(
        &self,
        client_updates: &[Vec<f64>],
        sensitivities: &[f64],
    ) -> (Vec<f64>, (f64, f64)) {
        if client_updates.is_empty() {
            return (vec![], (0.0, 0.0));
        }

        let dimension = client_updates[0].len();
        
        // 1. Apply local DP to each client's update
        let locally_perturbed: Vec<Vec<f64>> = client_updates
            .iter()
            .zip(sensitivities.iter())
            .map(|(update, &sensitivity)| {
                self.dp_engine.gaussian_mechanism(update, sensitivity)
            })
            .collect();
        
        // 2. Securely aggregate (simulated)
        let mut aggregated = vec![0.0; dimension];
        for update in &locally_perturbed {
            for (i, &val) in update.iter().enumerate() {
                aggregated[i] += val;
            }
        }
        
        let count = locally_perturbed.len() as f64;
        for val in &mut aggregated {
            *val /= count;
        }
        
        // 3. Apply privacy amplification if enabled
        let mut privacy_cost = if self.amplification_enabled {
            let sampling_rate = 1.0 / self.participants as f64;
            let amplifier = PrivacyAmplification::new(sampling_rate);
            let base_cost = (self.dp_engine.config.epsilon, self.dp_engine.config.delta);
            amplifier.amplify(base_cost.0, base_cost.1)
        } else {
            (self.dp_engine.config.epsilon, self.dp_engine.config.delta)
        };
        
        // 4. Adjust for distributed setting
        privacy_cost.0 /= (self.threshold as f64).sqrt();
        
        (aggregated, privacy_cost)
    }

    /// Enable or disable privacy amplification.
    pub fn set_amplification_enabled(&mut self, enabled: bool) {
        self.amplification_enabled = enabled;
    }
}

/// Differential privacy with adaptive noise scaling.
pub struct AdaptiveNoiseScaling {
    /// Base noise scale.
    base_sigma: f64,
    /// Adaptation rate.
    adaptation_rate: f64,
    /// Target privacy budget.
    target_epsilon: f64,
    /// Current iteration.
    iteration: usize,
}

impl AdaptiveNoiseScaling {
    /// Create a new adaptive noise scaler.
    pub fn new(base_sigma: f64, adaptation_rate: f64, target_epsilon: f64) -> Self {
        Self {
            base_sigma,
            adaptation_rate,
            target_epsilon,
            iteration: 0,
        }
    }

    /// Compute adaptive noise scale for current iteration.
    pub fn adaptive_sigma(&mut self, current_epsilon: f64) -> f64 {
        self.iteration += 1;
        
        // Adjust sigma based on how close we are to target
        let error_ratio = current_epsilon / self.target_epsilon;
        let adjustment = if error_ratio > 1.0 {
            // Over budget, increase noise
            1.0 + self.adaptation_rate * (error_ratio - 1.0)
        } else {
            // Under budget, can reduce noise slightly
            1.0 / (1.0 + self.adaptation_rate * (1.0 - error_ratio))
        };
        
        self.base_sigma * adjustment
    }

    /// Reset adaptation state.
    pub fn reset(&mut self) {
        self.iteration = 0;
    }
}

/// Integration with federated learning pipeline.
pub mod federated_integration {
    use super::*;
    use crate::model::ModelUpdate;
    
    /// Federated learning with advanced differential privacy.
    pub struct FederatedLearningWithDP {
        /// DP engine.
        dp_engine: Arc<AdvancedDifferentialPrivacy>,
        /// Distributed DP engine.
        distributed_dp: Option<DistributedDifferentialPrivacy>,
        /// Privacy accountant.
        accountant: MomentsAccountant,
        /// Total privacy budget spent.
        total_privacy_cost: (f64, f64),
    }
    
    impl FederatedLearningWithDP {
        /// Create a new federated learning DP wrapper.
        pub fn new(
            dp_config: crate::privacy::DifferentialPrivacyConfig,
            participants: Option<usize>,
        ) -> Self {
            let dp_engine = Arc::new(AdvancedDifferentialPrivacy::new(dp_config.clone(), None));
            
            let distributed_dp = participants.map(|p| {
                DistributedDifferentialPrivacy::new(
                    dp_engine.clone(),
                    p,
                    (p * 2 / 3).max(1), // 2/3 threshold
                )
            });
            
            Self {
                dp_engine,
                distributed_dp,
                accountant: MomentsAccountant::new(dp_config),
                total_privacy_cost: (0.0, 0.0),
            }
        }
        
        /// Apply DP to a federated learning round.
        pub fn apply_round(
            &mut self,
            client_updates: &[ModelUpdate],
            learning_rate: f64,
            batch_size: usize,
            total_samples: usize,
        ) -> (ModelUpdate, (f64, f64)) {
            // Convert model updates to gradients
            let gradients: Vec<Vec<f64>> = client_updates
                .iter()
                .map(|update| update.parameters.clone())
                .collect();
            
            let (update_gradient, round_cost) = if let Some(dist_dp) = &self.distributed_dp {
                let sensitivities = vec![self.dp_engine.config.clip_norm; gradients.len()];
                let (agg_gradient, cost) = dist_dp.apply_distributed_dp(&gradients, &sensitivities);
                (agg_gradient, cost)
            } else {
                // Use DP-SGD
                self.dp_engine.dp_sgd(
                    &gradients,
                    learning_rate,
                    batch_size,
                    total_samples,
                )
            };
            
            // Update total privacy cost
            self.total_privacy_cost.0 += round_cost.0;
            self.total_privacy_cost.1 += round_cost.1;
            
            // Update accountant
            self.accountant.update(
                batch_size,
                total_samples,
                1, // one iteration per round
            );
            
            let update = ModelUpdate {
                parameters: update_gradient,
                metadata: HashMap::from([
                    ("privacy_epsilon".to_string(), round_cost.0.to_string()),
                    ("privacy_delta".to_string(), round_cost.1.to_string()),
                    ("mechanism".to_string(), "advanced_dp".to_string()),
                ]),
            };
            
            (update, round_cost)
        }
        
        /// Get total privacy cost so far.
        pub fn total_privacy_cost(&self) -> (f64, f64) {
            self.total_privacy_cost
        }
        
        /// Check if privacy budget is exhausted.
        pub fn is_budget_exhausted(&self, max_epsilon: f64, max_delta: f64) -> bool {
            self.total_privacy_cost.0 >= max_epsilon || 
            self.total_privacy_cost.1 >= max_delta
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gaussian_mechanism() {
        let config = crate::privacy::DifferentialPrivacyConfig {
            epsilon: 1.0,
            delta: 1e-5,
            sigma: 1.0,
            clip_norm: 1.0,
        };
        
        let dp = AdvancedDifferentialPrivacy::new(config, None);
        let vector = vec![1.0, 2.0, 3.0];
        let sensitivity = 1.0;
        
        let noisy = dp.gaussian_mechanism(&vector, sensitivity);
        
        assert_eq!(noisy.len(), 3);
        // Noise should be added, so values should be different
        assert!(noisy[0] != 1.0 || noisy[1] != 2.0 || noisy[2] != 3.0);
    }
    
    #[test]
    fn test_adaptive_clipping() {
        let config = crate::privacy::DifferentialPrivacyConfig::default();
        let dp = AdvancedDifferentialPrivacy::new(config, None);
        
        let vectors = vec![
            vec![1.0, 0.0],
            vec![2.0, 0.0],
            vec![3.0, 0.0],
            vec![4.0, 0.0],
            vec![5.0, 0.0],
        ];
        
        let clip_norm = dp.adaptive_clip(&vectors, 80.0); // 80th percentile
        
        // 80th percentile of norms [1, 2, 3, 4, 5] is 4
        assert!((clip_norm - 4.0).abs() < 0.1);
    }
    
    #[test]
    fn test_moments_accountant() {
        let config = crate::privacy::DifferentialPrivacyConfig {
            epsilon: 1.0,
            delta: 1e-5,
            sigma: 1.0,
            clip_norm: 1.0,
        };
        
        let mut accountant = MomentsAccountant::new(config);
        accountant.update(100, 1000, 10);
        
        let cost = accountant.compute_privacy_cost(100, 1000, 10);
        assert!(cost.0 > 0.0);
        assert!(cost.1 > 0.0);
    }
    
    #[test]
    fn test_privacy_amplification() {
        let amplifier = PrivacyAmplification::new(0.1); // 10% sampling rate
        let (epsilon, delta) = amplifier.amplify(1.0, 1e-5);
        
        // Amplification should reduce epsilon
        assert!(epsilon < 1.0);
        assert!(delta < 1e-5);
    }
}