//! Proof‑of‑stake logic.

use crate::error::{Error, Result};
use rand::Rng;
use std::collections::HashMap;

/// Manages stake distribution and validator selection.
pub struct StakeManager {
    /// Map from validator public key (bytes) to stake amount.
    stakes: HashMap<Vec<u8>, u64>,
    /// Total stake across all validators.
    total_stake: u64,
}

impl StakeManager {
    pub fn new() -> Self {
        Self {
            stakes: HashMap::new(),
            total_stake: 0,
        }
    }

    /// Add or update stake for a validator.
    pub fn set_stake(&mut self, validator: Vec<u8>, amount: u64) {
        let old = self.stakes.insert(validator.clone(), amount).unwrap_or(0);
        self.total_stake = self.total_stake - old + amount;
    }

    /// Get stake for a validator.
    pub fn stake(&self, validator: &[u8]) -> u64 {
        self.stakes.get(validator).cloned().unwrap_or(0)
    }

    /// Select a validator pseudo‑randomly proportional to stake.
    pub fn select_validator(&self) -> Option<Vec<u8>> {
        if self.total_stake == 0 {
            return None;
        }
        let mut rng = rand::thread_rng();
        let mut point = rng.gen_range(0..self.total_stake);
        for (validator, &stake) in &self.stakes {
            if point < stake {
                return Some(validator.clone());
            }
            point -= stake;
        }
        None
    }

    /// Get total stake.
    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }
}

/// Proof‑of‑stake consensus engine.
pub struct ProofOfStake {
    stake_manager: StakeManager,
    /// Minimum stake required to be a validator.
    min_stake: u64,
}

impl ProofOfStake {
    pub fn new(min_stake: u64) -> Self {
        Self {
            stake_manager: StakeManager::new(),
            min_stake,
        }
    }

    /// Check if a validator is eligible to propose a block.
    pub fn can_propose(&self, validator: &[u8]) -> bool {
        self.stake_manager.stake(validator) >= self.min_stake
    }

    /// Select the next block proposer.
    pub fn select_proposer(&self) -> Result<Vec<u8>> {
        self.stake_manager.select_validator()
            .ok_or_else(|| Error::Consensus("No validators with stake".to_string()))
    }

    /// Add stake for a validator.
    pub fn add_stake(&mut self, validator: Vec<u8>, amount: u64) {
        self.stake_manager.set_stake(validator, amount);
    }

    /// Get stake manager.
    pub fn stake_manager(&self) -> &StakeManager {
        &self.stake_manager
    }
}