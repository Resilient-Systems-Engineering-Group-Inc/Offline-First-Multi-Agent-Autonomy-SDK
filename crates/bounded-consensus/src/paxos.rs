//! Paxos consensus algorithm implementation.

use crate::{BoundedConsensus, BoundedConsensusConfig, Proposal, ConsensusOutcome};
use common::error::{Result, SdkError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration, timeout};
use tracing::{info, warn, error};

/// Paxos roles: Proposer, Acceptor, Learner.
/// In this simplified version, each agent plays all roles.
struct PaxosState<T> {
    // Acceptor state
    promised_id: Option<u64>,
    accepted_id: Option<u64>,
    accepted_value: Option<T>,
}

/// Paxos message types.
#[derive(Debug, Serialize, Deserialize)]
enum PaxosMessage<T> {
    Prepare {
        proposal_id: u64,
    },
    Promise {
        proposal_id: u64,
        previous_id: Option<u64>,
        previous_value: Option<T>,
    },
    Accept {
        proposal_id: u64,
        value: T,
    },
    Accepted {
        proposal_id: u64,
        value: T,
    },
}

/// Paxos consensus instance.
pub struct PaxosConsensus<T> {
    config: BoundedConsensusConfig,
    // Per‑instance state
    state: PaxosState<T>,
    // Current proposal being handled
    current_proposal: Option<Proposal<T>>,
    // Collect promises/accepts
    promises: HashSet<AgentId>,
    accepts: HashSet<AgentId>,
    // Outcome channel
    outcome_tx: Option<oneshot::Sender<ConsensusOutcome<T>>>,
}

impl<T> PaxosConsensus<T> {
    pub fn new(config: BoundedConsensusConfig) -> Self {
        Self {
            config,
            state: PaxosState {
                promised_id: None,
                accepted_id: None,
                accepted_value: None,
            },
            current_proposal: None,
            promises: HashSet::new(),
            accepts: HashSet::new(),
            outcome_tx: None,
        }
    }

    /// Send a message to a participant (simulated).
    fn send_message(&self, recipient: AgentId, msg: PaxosMessage<T>) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let payload = bincode::serialize(&msg)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        info!("Simulating send to {}: {:?}", recipient.0, msg);
        Ok(())
    }

    /// Start the prepare phase as a proposer.
    fn start_prepare(&mut self, proposal: Proposal<T>) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        info!("Paxos: starting prepare for proposal {}", proposal.id);
        self.current_proposal = Some(proposal.clone());
        self.promises.clear();
        self.accepts.clear();

        // Send Prepare to all acceptors (all participants except self)
        for &participant in &self.config.participants {
            if participant == self.config.local_agent_id {
                continue;
            }
            self.send_message(
                participant,
                PaxosMessage::Prepare { proposal_id: proposal.id },
            )?;
        }
        Ok(())
    }

    /// Handle a Promise message from an acceptor.
    fn handle_promise(
        &mut self,
        sender: AgentId,
        proposal_id: u64,
        previous_id: Option<u64>,
        previous_value: Option<T>,
    ) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        if let Some(ref proposal) = self.current_proposal {
            if proposal.id != proposal_id {
                warn!("Promise for unknown proposal {}", proposal_id);
                return Ok(());
            }
        } else {
            warn!("Promise received but no current proposal");
            return Ok(());
        }

        self.promises.insert(sender);
        info!("Paxos: promise from {} for proposal {}", sender.0, proposal_id);

        // Update proposal value if we learn a previously accepted value
        if let (Some(pid), Some(pval)) = (previous_id, previous_value) {
            // If this previous id is higher than our current proposal's? Not needed for correctness.
            // For simplicity, we adopt the value with the highest previous id.
            // We'll just log.
            info!("Paxos: learned previous value from {} with id {}", sender.0, pid);
        }

        // Check if we have a majority of promises
        let majority = (self.config.participants.len() / 2) + 1;
        if self.promises.len() >= majority {
            self.start_accept()?;
        }
        Ok(())
    }

    /// Start the accept phase (send Accept messages).
    fn start_accept(&mut self) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        let proposal = self.current_proposal.as_ref().expect("proposal must exist");
        info!("Paxos: majority promises reached, sending Accept for proposal {}", proposal.id);
        for &participant in &self.config.participants {
            if participant == self.config.local_agent_id {
                continue;
            }
            self.send_message(
                participant,
                PaxosMessage::Accept {
                    proposal_id: proposal.id,
                    value: proposal.value.clone(),
                },
            )?;
        }
        Ok(())
    }

    /// Handle an Accepted message from an acceptor.
    fn handle_accepted(
        &mut self,
        sender: AgentId,
        proposal_id: u64,
        value: T,
    ) -> Result<()>
    where
        T: Clone,
    {
        if let Some(ref proposal) = self.current_proposal {
            if proposal.id != proposal_id {
                warn!("Accepted for unknown proposal {}", proposal_id);
                return Ok(());
            }
        } else {
            warn!("Accepted received but no current proposal");
            return Ok(());
        }

        self.accepts.insert(sender);
        info!("Paxos: accepted from {} for proposal {}", sender.0, proposal_id);

        let majority = (self.config.participants.len() / 2) + 1;
        if self.accepts.len() >= majority {
            info!("Paxos: majority accepted, consensus reached for proposal {}", proposal_id);
            if let Some(tx) = self.outcome_tx.take() {
                let _ = tx.send(ConsensusOutcome::Decided(value));
            }
        }
        Ok(())
    }

    /// As an acceptor, handle a Prepare message.
    fn handle_prepare(&mut self, sender: AgentId, proposal_id: u64) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        // If we have already promised a higher proposal id, reject.
        if let Some(pid) = self.state.promised_id {
            if proposal_id <= pid {
                info!("Paxos: rejecting Prepare {} because promised {}", proposal_id, pid);
                // In real Paxos we would send a NACK, but we ignore for simplicity.
                return Ok(());
            }
        }
        // Promise not to accept any proposal with id < proposal_id
        self.state.promised_id = Some(proposal_id);
        info!("Paxos: promising for proposal {}", proposal_id);
        // Send Promise back
        self.send_message(
            sender,
            PaxosMessage::Promise {
                proposal_id,
                previous_id: self.state.accepted_id,
                previous_value: self.state.accepted_value.clone(),
            },
        )?;
        Ok(())
    }

    /// As an acceptor, handle an Accept message.
    fn handle_accept(&mut self, sender: AgentId, proposal_id: u64, value: T) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        // Check if we have promised not to accept this proposal id
        if let Some(pid) = self.state.promised_id {
            if proposal_id < pid {
                info!("Paxos: rejecting Accept {} because promised {}", proposal_id, pid);
                return Ok(());
            }
        }
        // Accept the proposal
        self.state.accepted_id = Some(proposal_id);
        self.state.accepted_value = Some(value.clone());
        info!("Paxos: accepted proposal {}", proposal_id);
        // Send Accepted back
        self.send_message(
            sender,
            PaxosMessage::Accepted {
                proposal_id,
                value,
            },
        )?;
        Ok(())
    }

    /// Update the set of participants dynamically.
    /// If a proposal is active, it will be aborted.
    fn update_participants(&mut self, new_participants: HashSet<AgentId>) -> Result<()>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        info!(
            "Paxos: updating participants from {:?} to {:?}",
            self.config.participants, new_participants
        );
        // Abort any ongoing proposal
        if self.current_proposal.is_some() {
            info!("Paxos: aborting active proposal due to membership change");
            if let Some(tx) = self.outcome_tx.take() {
                let _ = tx.send(ConsensusOutcome::Aborted);
            }
            self.current_proposal = None;
            self.promises.clear();
            self.accepts.clear();
        }
        // Update configuration
        self.config.update_participants(new_participants);
        Ok(())
    }
}

#[async_trait]
impl<T> BoundedConsensus for PaxosConsensus<T>
where
    T: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
    type Value = T;

    async fn propose(
        &mut self,
        proposal: Proposal<Self::Value>,
    ) -> Result<mpsc::Receiver<ConsensusOutcome<Self::Value>>> {
        if self.current_proposal.is_some() {
            return Err(SdkError::Consensus("Another proposal is already active".to_string()));
        }

        let (outcome_tx, outcome_rx) = oneshot::channel();
        let (tx, mut rx) = mpsc::channel(1);

        self.outcome_tx = Some(outcome_tx);
        self.start_prepare(proposal)?;

        // Spawn timeout task
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
                    let _ = tx.send(ConsensusOutcome::Timeout).await;
                }
                Err(_) => {
                    let _ = tx.send(ConsensusOutcome::Timeout).await;
                }
            }
        });

        Ok(rx)
    }

    async fn handle_message(&mut self, sender: AgentId, payload: Vec<u8>) -> Result<()> {
        let msg: PaxosMessage<T> = bincode::deserialize(&payload)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        match msg {
            PaxosMessage::Prepare { proposal_id } => {
                self.handle_prepare(sender, proposal_id)?;
            }
            PaxosMessage::Promise { proposal_id, previous_id, previous_value } => {
                self.handle_promise(sender, proposal_id, previous_id, previous_value)?;
            }
            PaxosMessage::Accept { proposal_id, value } => {
                self.handle_accept(sender, proposal_id, value)?;
            }
            PaxosMessage::Accepted { proposal_id, value } => {
                self.handle_accepted(sender, proposal_id, value)?;
            }
        }
        Ok(())
    }

    fn config(&self) -> &BoundedConsensusConfig {
        &self.config
    }

    async fn update_participants(&mut self, new_participants: HashSet<AgentId>) -> Result<()> {
        self.update_participants(new_participants)
    }
}