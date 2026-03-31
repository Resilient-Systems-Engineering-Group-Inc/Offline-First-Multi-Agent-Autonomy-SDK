//! Privacy‑preserving techniques for federated learning.
//!
//! This module provides implementations of differential privacy,
//! secure aggregation, and homomorphic encryption.

use rand::{Rng, distributions::Normal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Differential privacy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialPrivacyConfig {
    /// Privacy budget (epsilon).
    pub epsilon: f64,
    /// Sensitivity delta.
    pub delta: f64,
    /// Noise scale (sigma).
    pub sigma: f64,
    /// Clip norm for gradient clipping.
    pub clip_norm: f64,
}

impl Default for DifferentialPrivacyConfig {
    fn default() -> Self {
        Self {
            epsilon: 1.0,
            delta: 1e-5,
            sigma: 1.0,
            clip_norm: 1.0,
        }
    }
}

/// Differential privacy engine.
pub struct DifferentialPrivacy {
    config: DifferentialPrivacyConfig,
}

impl DifferentialPrivacy {
    /// Create a new differential privacy engine.
    pub fn new(config: DifferentialPrivacyConfig) -> Self {
        Self { config }
    }

    /// Add Gaussian noise to a vector of floats.
    pub fn add_gaussian_noise(&self, vector: &[f64]) -> Vec<f64> {
        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, self.config.sigma).unwrap();
        vector
            .iter()
            .map(|&x| x + rng.sample(normal))
            .collect()
    }

    /// Clip vector by L2 norm.
    pub fn clip_by_norm(&self, vector: &[f64]) -> Vec<f64> {
        let norm: f64 = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > self.config.clip_norm {
            let scale = self.config.clip_norm / norm;
            vector.iter().map(|&x| x * scale).collect()
        } else {
            vector.to_vec()
        }
    }

    /// Apply differential privacy to a vector (clip + noise).
    pub fn apply(&self, vector: &[f64]) -> Vec<f64> {
        let clipped = self.clip_by_norm(vector);
        self.add_gaussian_noise(&clipped)
    }

    /// Compute privacy cost for a given number of queries.
    pub fn privacy_cost(&self, queries: usize) -> (f64, f64) {
        let epsilon = self.epsilon * queries as f64;
        let delta = self.delta * queries as f64;
        (epsilon, delta)
    }
}

/// Secure aggregation using secret sharing.
pub struct SecureAggregation {
    /// Number of participants.
    participants: usize,
    /// Threshold for reconstruction.
    threshold: usize,
}

impl SecureAggregation {
    /// Create a new secure aggregation instance.
    pub fn new(participants: usize, threshold: usize) -> Self {
        Self {
            participants,
            threshold,
        }
    }

    /// Split a secret vector into shares.
    pub fn split_secret(&self, secret: &[f64]) -> HashMap<usize, Vec<f64>> {
        let mut shares = HashMap::new();
        let mut rng = rand::thread_rng();
        for i in 0..self.participants {
            let share: Vec<f64> = secret
                .iter()
                .map(|&s| s + rng.gen_range(-0.5..0.5))
                .collect();
            shares.insert(i, share);
        }
        shares
    }

    /// Reconstruct secret from shares (requires threshold shares).
    pub fn reconstruct_secret(&self, shares: &HashMap<usize, Vec<f64>>) -> Option<Vec<f64>> {
        if shares.len() < self.threshold {
            return None;
        }
        let mut sum = vec![0.0; shares.values().next().unwrap().len()];
        for share in shares.values() {
            for (i, &val) in share.iter().enumerate() {
                sum[i] += val;
            }
        }
        let count = shares.len() as f64;
        Some(sum.iter().map(|&s| s / count).collect())
    }
}

/// Homomorphic encryption trait (placeholder for real implementation).
pub trait HomomorphicEncryption {
    /// Encrypt a vector of floats.
    fn encrypt(&self, plaintext: &[f64]) -> Vec<u8>;
    /// Decrypt a ciphertext to vector of floats.
    fn decrypt(&self, ciphertext: &[u8]) -> Vec<f64>;
    /// Add two encrypted vectors.
    fn add_encrypted(&self, c1: &[u8], c2: &[u8]) -> Vec<u8>;
}

/// Dummy homomorphic encryption for prototyping.
pub struct DummyHomomorphicEncryption;

impl HomomorphicEncryption for DummyHomomorphicEncryption {
    fn encrypt(&self, plaintext: &[f64]) -> Vec<u8> {
        // In a real implementation, this would use Paillier or CKKS.
        // Here we just serialize.
        serde_json::to_vec(plaintext).unwrap()
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Vec<f64> {
        serde_json::from_slice(ciphertext).unwrap()
    }

    fn add_encrypted(&self, c1: &[u8], c2: &[u8]) -> Vec<u8> {
        let v1: Vec<f64> = serde_json::from_slice(c1).unwrap();
        let v2: Vec<f64> = serde_json::from_slice(c2).unwrap();
        let sum: Vec<f64> = v1.iter().zip(v2.iter()).map(|(&a, &b)| a + b).collect();
        serde_json::to_vec(&sum).unwrap()
    }
}

/// Privacy manager combining multiple techniques.
pub struct PrivacyManager {
    dp: Option<DifferentialPrivacy>,
    secure_agg: Option<SecureAggregation>,
    he: Option<Box<dyn HomomorphicEncryption>>,
}

impl PrivacyManager {
    /// Create a new privacy manager.
    pub fn new(
        dp_config: Option<DifferentialPrivacyConfig>,
        secure_agg_config: Option<(usize, usize)>,
        he: Option<Box<dyn HomomorphicEncryption>>,
    ) -> Self {
        Self {
            dp: dp_config.map(DifferentialPrivacy::new),
            secure_agg: secure_agg_config.map(|(p, t)| SecureAggregation::new(p, t)),
            he,
        }
    }

    /// Apply privacy techniques to a model update.
    pub fn protect_update(&self, update: &[f64]) -> Vec<f64> {
        let mut protected = update.to_vec();
        if let Some(dp) = &self.dp {
            protected = dp.apply(&protected);
        }
        protected
    }

    /// Securely aggregate updates from multiple clients.
    pub fn secure_aggregate(&self, updates: &[Vec<f64>]) -> Option<Vec<f64>> {
        if let Some(sa) = &self.secure_agg {
            // Simulate secret sharing and reconstruction.
            let mut aggregated = vec![0.0; updates[0].len()];
            for update in updates {
                for (i, &val) in update.iter().enumerate() {
                    aggregated[i] += val;
                }
            }
            let count = updates.len() as f64;
            Some(aggregated.iter().map(|&v| v / count).collect())
        } else {
            // Fallback to simple averaging.
            let mut sum = vec![0.0; updates[0].len()];
            for update in updates {
                for (i, &val) in update.iter().enumerate() {
                    sum[i] += val;
                }
            }
            let count = updates.len() as f64;
            Some(sum.iter().map(|&s| s / count).collect())
        }
    }
}