//! Bounded consensus for offline‑first multi‑agent systems.
//!
//! This module provides a consensus protocol that guarantees termination within a bounded
//! number of communication rounds, suitable for partially synchronous networks.

use common::types::AgentId;
use common::error::{Result, SdkError};
use common::metrics;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration, timeout};
use tracing::{info, warn, error};

/// A proposal that agents attempt to agree upon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal<T> {
    /// Unique identifier for this proposal (e.g., sequence number).
    pub id: u64,
    /// The value being proposed.
    pub value: T,
    /// The agent that originally proposed this value.
    pub proposer: AgentId,
}

/// Outcome of a consensus round.
#[derive(Debug, Clone)]
pub enum ConsensusOutcome<T> {
    /// Consensus reached with the decided value.
    Decided(T),
    /// Consensus not reached within the bound (timeout).
    Timeout,
    /// Consensus aborted due to contention.
    Aborted,
}

/// Configuration for bounded consensus.
#[derive(Debug, Clone)]
pub struct BoundedConsensusConfig {
    /// Local agent ID.
    pub local_agent_id: AgentId,
    /// Set of all participant agent IDs.
    pub participants: HashSet<AgentId>,
    /// Maximum number of communication rounds before timeout.
    pub max_rounds: u32,
    /// Round duration in milliseconds.
    pub round_duration_ms: u64,
}

/// Trait for a bounded consensus protocol.
#[async_trait]
pub trait BoundedConsensus: Send + Sync {
    /// Type of value being decided.
    type Value: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>;

    /// Start a new consensus round for a given proposal.
    /// Returns a channel receiver that will receive the outcome.
    async fn propose(
        &mut self,
        proposal: Proposal<Self::Value>,
    ) -> Result<mpsc::Receiver<ConsensusOutcome<Self::Value>>>;

    /// Handle an incoming message from another agent.
    async fn handle_message(&mut self, sender: AgentId, payload: Vec<u8>) -> Result<()>;

    /// Get the current configuration.
    fn config(&self) -> &BoundedConsensusConfig;
}

/// Internal state of a two‑phase commit round.
struct RoundState<T> {
    proposal: Proposal<T>,
    votes: HashMap<AgentId, bool>,
    phase: Phase,
    outcome_tx: Option<oneshot::Sender<ConsensusOutcome<T>>>,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Prepare,
    Voting,
    Committed,
    Aborted,
}

/// Two‑phase commit messages.
#[derive(Debug, Serialize, Deserialize)]
enum TwoPhaseMessage<T> {
    Prepare {
        proposal_id: u64,
        value: T,
        proposer: AgentId,
    },
    Vote {
        proposal_id: u64,
        vote: bool,
    },
    Commit {
        proposal_id: u64,
    },
    Abort {
        proposal_id: u64,
    },
    Ack {
        proposal_id: u64,
    },
}

/// A simple implementation of bounded consensus using a two‑phase commit.
pub struct TwoPhaseBoundedConsensus<T> {
    config: BoundedConsensusConfig,
    /// Current active round, if any.
    current_round: Option<RoundState<T>>,
    /// Channel to send network messages (for simulation, we'll just log).
    /// In a real implementation, this would be a transport.
    _network_tx: mpsc::UnboundedSender<(AgentId, Vec<u8>)>,
}

impl<T> TwoPhaseBoundedConsensus<T> {
    /// Create a new instance.
    pub fn new(config: BoundedConsensusConfig) -> Self {
        let (tx, _) = mpsc::unbounded_channel();
        Self {
            config,
            current_round: None,
            _network_tx: tx,
        }
    }

    /// Simulate sending a message to a participant.
    fn send_message(&self, recipient: AgentId, msg: TwoPhaseMessage<T>) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let payload = bincode::serialize(&msg)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        // In a real implementation, we would send via transport.
        info!("Simulating send to {}: {:?}", recipient.0, msg);
        Ok(())
    }

    /// Start the prepare phase.
    fn start_prepare(&mut self, proposal: Proposal<T>) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        info!("Starting prepare phase for proposal {}", proposal.id);
        let participants = self.config.participants.clone();
        // Send Prepare to all participants except ourselves
        for &participant in &participants {
            if participant == self.config.local_agent_id {
                continue;
            }
            self.send_message(
                participant,
                TwoPhaseMessage::Prepare {
                    proposal_id: proposal.id,
                    value: proposal.value.clone(),
                    proposer: proposal.proposer,
                },
            )?;
        }

        // Initialize round state
        self.current_round = Some(RoundState {
            proposal,
            votes: HashMap::new(),
            phase: Phase::Prepare,
            outcome_tx: None,
        });
        Ok(())
    }

    /// Process a vote from a participant.
    fn process_vote(&mut self, voter: AgentId, proposal_id: u64, vote: bool) -> Result<()>
    where
        T: Clone,
    {
        if let Some(round) = &mut self.current_round {
            if round.proposal.id != proposal_id {
                warn!("Vote for unknown proposal {}", proposal_id);
                return Ok(());
            }
            if round.phase != Phase::Prepare && round.phase != Phase::Voting {
                warn!("Vote received in wrong phase {:?}", round.phase);
                return Ok(());
            }
            round.votes.insert(voter, vote);
            info!("Vote from {}: {}", voter.0, vote);

            // Check if we have all votes
            let expected_voters: HashSet<_> = self
                .config
                .participants
                .iter()
                .filter(|&id| *id != self.config.local_agent_id)
                .cloned()
                .collect();
            let received_voters: HashSet<_> = round.votes.keys().cloned().collect();
            if expected_voters.is_subset(&received_voters) {
                self.finish_voting()?;
            }
        }
        Ok(())
    }

    /// Finish voting and decide commit/abort.
    fn finish_voting(&mut self) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        let round = self.current_round.as_mut().expect("round must exist");
        let all_yes = round.votes.values().all(|&v| v);
        if all_yes {
            info!("All votes YES, committing proposal {}", round.proposal.id);
            round.phase = Phase::Committed;
            // Send Commit to all participants
            for &participant in &self.config.participants {
                if participant == self.config.local_agent_id {
                    continue;
                }
                self.send_message(participant, TwoPhaseMessage::Commit { proposal_id: round.proposal.id })?;
            }
            // Notify outcome
            if let Some(tx) = round.outcome_tx.take() {
                let _ = tx.send(ConsensusOutcome::Decided(round.proposal.value.clone()));
            }
            metrics::inc_consensus_rounds_completed();
        } else {
            info!("Some votes NO, aborting proposal {}", round.proposal.id);
            round.phase = Phase::Aborted;
            for &participant in &self.config.participants {
                if participant == self.config.local_agent_id {
                    continue;
                }
                self.send_message(participant, TwoPhaseMessage::Abort { proposal_id: round.proposal.id })?;
            }
            if let Some(tx) = round.outcome_tx.take() {
                let _ = tx.send(ConsensusOutcome::Aborted);
            }
            metrics::inc_consensus_rounds_completed();
        }
        Ok(())
    }
}

#[async_trait]
impl<T> BoundedConsensus for TwoPhaseBoundedConsensus<T>
where
    T: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
    type Value = T;

    async fn propose(
        &mut self,
        proposal: Proposal<Self::Value>,
    ) -> Result<mpsc::Receiver<ConsensusOutcome<Self::Value>>> {
        // Ensure no active round
        if self.current_round.is_some() {
            return Err(SdkError::Consensus("Another round is already active".to_string()));
        }

        metrics::inc_consensus_rounds_started();

        let (outcome_tx, outcome_rx) = oneshot::channel();
        let (tx, mut rx) = mpsc::channel(1);

        // Store outcome sender in round state
        self.start_prepare(proposal.clone())?;
        if let Some(round) = &mut self.current_round {
            round.outcome_tx = Some(outcome_tx);
        }

        // Spawn a task to handle timeout
        let config = self.config.clone();
        let proposal_id = proposal.id;
        tokio::spawn(async move {
            let timeout_duration = Duration::from_millis(config.round_duration_ms * config.max_rounds as u64);
            match timeout(timeout_duration, async {
                outcome_rx.await.ok()
            }).await {
                Ok(Some(outcome)) => {
                    let _ = tx.send(outcome).await;
                }
                Ok(None) => {
                    metrics::inc_consensus_rounds_completed();
                    let _ = tx.send(ConsensusOutcome::Timeout).await;
                }
                Err(_) => {
                    metrics::inc_consensus_rounds_completed();
                    let _ = tx.send(ConsensusOutcome::Timeout).await;
                }
            }
        });

        Ok(rx)
    }

    async fn handle_message(&mut self, sender: AgentId, payload: Vec<u8>) -> Result<()> {
        let msg: TwoPhaseMessage<T> = bincode::deserialize(&payload)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        match msg {
            TwoPhaseMessage::Prepare { proposal_id, value, proposer } => {
                // As a participant, we receive a prepare request.
                // For simplicity, we always vote YES.
                info!("Received Prepare for proposal {} from {}", proposal_id, proposer.0);
                let vote_msg = TwoPhaseMessage::Vote {
                    proposal_id,
                    vote: true,
                };
                let vote_payload = bincode::serialize(&vote_msg)
                    .map_err(|e| SdkError::Serialization(e.to_string()))?;
                // In real implementation, we would send back to proposer via transport.
                // Here we just simulate by calling handle_message recursively (not good).
                // Instead, we'll just log.
                info!("Simulating YES vote to {}", proposer.0);
                // For demo, we can directly process the vote as if it arrived.
                self.process_vote(sender, proposal_id, true)?;
            }
            TwoPhaseMessage::Vote { proposal_id, vote } => {
                self.process_vote(sender, proposal_id, vote)?;
            }
            TwoPhaseMessage::Commit { proposal_id } => {
                info!("Received Commit for proposal {}", proposal_id);
                if let Some(round) = &mut self.current_round {
                    if round.proposal.id == proposal_id {
                        round.phase = Phase::Committed;
                        if let Some(tx) = round.outcome_tx.take() {
                            let _ = tx.send(ConsensusOutcome::Decided(round.proposal.value.clone()));
                        }
                        metrics::inc_consensus_rounds_completed();
                    }
                }
            }
            TwoPhaseMessage::Abort { proposal_id } => {
                info!("Received Abort for proposal {}", proposal_id);
                if let Some(round) = &mut self.current_round {
                    if round.proposal.id == proposal_id {
                        round.phase = Phase::Aborted;
                        if let Some(tx) = round.outcome_tx.take() {
                            let _ = tx.send(ConsensusOutcome::Aborted);
                        }
                        metrics::inc_consensus_rounds_completed();
                    }
                }
            }
            TwoPhaseMessage::Ack { .. } => {
                // Ignore acks for now
            }
        }
        Ok(())
    }

    fn config(&self) -> &BoundedConsensusConfig {
        &self.config
    }
mod paxos;
pub use paxos::PaxosConsensus;
}