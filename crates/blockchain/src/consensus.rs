//! Decentralized consensus mechanisms.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

/// Consensus algorithm types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusAlgorithm {
    ProofOfWork,
    ProofOfStake,
    PracticalByzantineFaultTolerance,
    Raft,
    Custom(String),
}

/// Consensus manager.
pub struct ConsensusManager {
    algorithm: ConsensusAlgorithm,
    participants: RwLock<HashMap<String, Participant>>,
    current_round: RwLock<u64>,
    votes: RwLock<HashMap<u64, HashMap<String, Vote>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub stake: u64,
    pub reputation: f64,
    pub last_vote: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub participant_id: String,
    pub block_hash: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl ConsensusManager {
    /// Create new consensus manager.
    pub fn new(algorithm: ConsensusAlgorithm) -> Self {
        Self {
            algorithm,
            participants: RwLock::new(HashMap::new()),
            current_round: RwLock::new(0),
            votes: RwLock::new(HashMap::new()),
        }
    }

    /// Add participant.
    pub async fn add_participant(&self, id: &str, stake: u64) {
        let mut participants = self.participants.write().await;
        participants.insert(
            id.to_string(),
            Participant {
                id: id.to_string(),
                stake,
                reputation: 1.0,
                last_vote: None,
            },
        );
        info!("Participant added: {} with stake {}", id, stake);
    }

    /// Submit vote.
    pub async fn submit_vote(&self, participant_id: &str, block_hash: &str) -> Result<()> {
        let round = *self.current_round.read().await;
        
        let mut votes = self.votes.write().await;
        let round_votes = votes.entry(round).or_insert_with(HashMap::new);
        
        // Create vote (simplified - no actual signature)
        let vote = Vote {
            participant_id: participant_id.to_string(),
            block_hash: block_hash.to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: vec![],
        };
        
        round_votes.insert(participant_id.to_string(), vote);
        
        // Update participant
        let mut participants = self.participants.write().await;
        if let Some(participant) = participants.get_mut(participant_id) {
            participant.last_vote = Some(round);
        }
        
        info!("Vote submitted by {} for round {}", participant_id, round);
        Ok(())
    }

    /// Check consensus.
    pub async fn check_consensus(&self, block_hash: &str) -> Result<bool> {
        let round = *self.current_round.read().await;
        let votes = self.votes.read().await;
        
        if let Some(round_votes) = votes.get(&round) {
            // Count votes for this block
            let vote_count = round_votes.values()
                .filter(|v| v.block_hash == block_hash)
                .count();
            
            let participants = self.participants.read().await;
            let total_participants = participants.len();
            
            // Need 2/3 majority for Byzantine fault tolerance
            let threshold = (total_participants * 2 / 3) + 1;
            
            Ok(vote_count >= threshold)
        } else {
            Ok(false)
        }
    }

    /// Advance to next round.
    pub async fn next_round(&self) -> u64 {
        let mut round = self.current_round.write().await;
        *round += 1;
        
        // Clear old votes
        let mut votes = self.votes.write().await;
        votes.retain(|&r, _| r >= *round - 10);
        
        info!("Advanced to round {}", *round);
        *round
    }

    /// Get leader for current round (Proof of Stake).
    pub async fn get_leader(&self) -> Option<String> {
        let participants = self.participants.read().await;
        
        if participants.is_empty() {
            return None;
        }
        
        // Weighted random selection based on stake
        let total_stake: u64 = participants.values().map(|p| p.stake).sum();
        let mut rng = rand::thread_rng();
        let mut selection = rng.gen_range(0..total_stake);
        
        for (id, participant) in participants.iter() {
            if selection < participant.stake {
                return Some(id.clone());
            }
            selection -= participant.stake;
        }
        
        participants.keys().next().cloned()
    }

    /// Calculate stake weight.
    pub fn calculate_stake_weight(&self, participant: &Participant) -> f64 {
        let time_weight = if let Some(last_vote) = participant.last_vote {
            let now = chrono::Utc::now().timestamp() as u64;
            let hours_since_vote = (now - last_vote) / 3600;
            
            // Decay weight over time
            1.0 / (1.0 + (hours_since_vote as f64 * 0.1))
        } else {
            1.0
        };
        
        participant.stake as f64 * participant.reputation * time_weight
    }

    /// Get consensus statistics.
    pub async fn get_stats(&self) -> ConsensusStats {
        let participants = self.participants.read().await;
        let current_round = *self.current_round.read().await;
        let votes = self.votes.read().await;
        
        let total_stake: u64 = participants.values().map(|p| p.stake).sum();
        let active_participants = participants.values()
            .filter(|p| p.last_vote.is_some())
            .count();
        
        ConsensusStats {
            algorithm: format!("{:?}", self.algorithm),
            current_round,
            total_participants: participants.len() as i64,
            active_participants: active_participants as i32,
            total_stake,
            total_votes: votes.values().map(|v| v.len()).sum::<usize>() as i64,
        }
    }
}

/// Consensus statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub algorithm: String,
    pub current_round: u64,
    pub total_participants: i64,
    pub active_participants: i32,
    pub total_stake: u64,
    pub total_votes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_manager() {
        let manager = ConsensusManager::new(ConsensusAlgorithm::PracticalByzantineFaultTolerance);

        // Add participants
        manager.add_participant("node-1", 100).await;
        manager.add_participant("node-2", 200).await;
        manager.add_participant("node-3", 150).await;

        // Submit votes
        manager.submit_vote("node-1", "block-abc").await.unwrap();
        manager.submit_vote("node-2", "block-abc").await.unwrap();
        manager.submit_vote("node-3", "block-def").await.unwrap();

        // Check consensus
        let has_consensus = manager.check_consensus("block-abc").await.unwrap();
        assert!(has_consensus);

        // Get stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_participants, 3);
    }

    #[tokio::test]
    async fn test_leader_selection() {
        let manager = ConsensusManager::new(ConsensusAlgorithm::ProofOfStake);

        manager.add_participant("node-1", 100).await;
        manager.add_participant("node-2", 900).await;

        let leader = manager.get_leader().await;
        assert!(leader.is_some());
    }
}
