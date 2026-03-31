//! Aggregation algorithms for federated learning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregation strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// Federated Averaging (FedAvg).
    FedAvg,
    /// Weighted averaging by number of samples.
    WeightedAvg,
    /// Secure aggregation with differential privacy.
    Secure,
    /// Median aggregation (robust to outliers).
    Median,
    /// Trimmed mean (discard extreme values).
    TrimmedMean(f64), // fraction to trim from each side
}

/// Configuration for aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Aggregation strategy.
    pub strategy: AggregationStrategy,
    /// Minimum number of clients required.
    pub min_clients: usize,
    /// Maximum number of clients allowed.
    pub max_clients: Option<usize>,
    /// Timeout for aggregation in seconds.
    pub timeout_secs: u64,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            strategy: AggregationStrategy::FedAvg,
            min_clients: 3,
            max_clients: None,
            timeout_secs: 30,
        }
    }
}

/// A client update with metadata.
#[derive(Debug, Clone)]
pub struct ClientUpdate {
    /// Client identifier.
    pub client_id: String,
    /// Model parameters as a vector of floats.
    pub parameters: Vec<f64>,
    /// Number of training samples used.
    pub sample_count: usize,
    /// Optional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Aggregator that combines client updates into a global model.
pub struct Aggregator {
    config: AggregationConfig,
}

impl Aggregator {
    /// Create a new aggregator.
    pub fn new(config: AggregationConfig) -> Self {
        Self { config }
    }

    /// Aggregate multiple client updates.
    pub fn aggregate(&self, updates: &[ClientUpdate]) -> Option<Vec<f64>> {
        if updates.len() < self.config.min_clients {
            return None;
        }

        match self.config.strategy {
            AggregationStrategy::FedAvg => self.fed_avg(updates),
            AggregationStrategy::WeightedAvg => self.weighted_avg(updates),
            AggregationStrategy::Secure => self.secure_aggregate(updates),
            AggregationStrategy::Median => self.median(updates),
            AggregationStrategy::TrimmedMean(trim) => self.trimmed_mean(updates, trim),
        }
    }

    /// Federated averaging (FedAvg).
    fn fed_avg(&self, updates: &[ClientUpdate]) -> Option<Vec<f64>> {
        self.weighted_avg(updates)
    }

    /// Weighted average by sample count.
    fn weighted_avg(&self, updates: &[ClientUpdate]) -> Option<Vec<f64>> {
        let total_samples: usize = updates.iter().map(|u| u.sample_count).sum();
        if total_samples == 0 {
            return None;
        }

        let dim = updates[0].parameters.len();
        let mut aggregated = vec![0.0; dim];

        for update in updates {
            let weight = update.sample_count as f64 / total_samples as f64;
            for (i, &param) in update.parameters.iter().enumerate() {
                aggregated[i] += param * weight;
            }
        }

        Some(aggregated)
    }

    /// Secure aggregation (placeholder).
    fn secure_aggregate(&self, updates: &[ClientUpdate]) -> Option<Vec<f64>> {
        // In a real implementation, this would use cryptographic techniques.
        // For now, fall back to weighted average.
        self.weighted_avg(updates)
    }

    /// Median aggregation (element‑wise median).
    fn median(&self, updates: &[ClientUpdate]) -> Option<Vec<f64>> {
        let dim = updates[0].parameters.len();
        let mut aggregated = Vec::with_capacity(dim);

        for i in 0..dim {
            let mut values: Vec<f64> = updates.iter().map(|u| u.parameters[i]).collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = values.len() / 2;
            let median = if values.len() % 2 == 0 {
                (values[mid - 1] + values[mid]) / 2.0
            } else {
                values[mid]
            };
            aggregated.push(median);
        }

        Some(aggregated)
    }

    /// Trimmed mean aggregation.
    fn trimmed_mean(&self, updates: &[ClientUpdate], trim_fraction: f64) -> Option<Vec<f64>> {
        let dim = updates[0].parameters.len();
        let mut aggregated = Vec::with_capacity(dim);

        for i in 0..dim {
            let mut values: Vec<f64> = updates.iter().map(|u| u.parameters[i]).collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let trim_count = (values.len() as f64 * trim_fraction).floor() as usize;
            let start = trim_count;
            let end = values.len() - trim_count;

            if start >= end {
                return None;
            }

            let trimmed = &values[start..end];
            let sum: f64 = trimmed.iter().sum();
            let mean = sum / trimmed.len() as f64;
            aggregated.push(mean);
        }

        Some(aggregated)
    }

    /// Validate updates (check dimensions, etc.).
    pub fn validate_updates(&self, updates: &[ClientUpdate]) -> Result<(), String> {
        if updates.is_empty() {
            return Err("No updates provided".to_string());
        }
        let dim = updates[0].parameters.len();
        for (idx, update) in updates.iter().enumerate() {
            if update.parameters.len() != dim {
                return Err(format!(
                    "Update {} has dimension {} but expected {}",
                    idx,
                    update.parameters.len(),
                    dim
                ));
            }
        }
        Ok(())
    }
}