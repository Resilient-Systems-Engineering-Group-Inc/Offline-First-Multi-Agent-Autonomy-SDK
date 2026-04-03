//! Distributed federated learning coordination for multi‑agent systems.
//!
//! This module provides integration with the mesh transport and state sync
//! to enable decentralized federated learning across agents.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::error::Error;
use crate::model::Model;
use crate::aggregation::{AggregationConfig, AggregationStrategy, ClientUpdate};
use crate::client::FederatedClient;
use crate::server::FederatedServer;

/// Event emitted by the distributed coordinator.
#[derive(Debug, Clone)]
pub enum DistributedTrainingEvent {
    /// A new round of training has started.
    RoundStarted {
        round_id: u64,
        participants: usize,
        model_version: u64,
    },
    /// A client has submitted an update.
    UpdateReceived {
        client_id: String,
        round_id: u64,
        samples: usize,
    },
    /// Aggregation completed.
    AggregationCompleted {
        round_id: u64,
        aggregated_model_version: u64,
        participant_count: usize,
    },
    /// Training round failed.
    RoundFailed {
        round_id: u64,
        reason: String,
    },
}

/// Configuration for distributed federated learning.
#[derive(Debug, Clone)]
pub struct DistributedTrainingConfig {
    /// Minimum number of agents required to start a round.
    pub min_agents: usize,
    /// Maximum number of agents to include.
    pub max_agents: Option<usize>,
    /// Rounds to run before stopping (None = infinite).
    pub max_rounds: Option<u64>,
    /// Timeout per round in seconds.
    pub round_timeout_secs: u64,
    /// Aggregation configuration.
    pub aggregation: AggregationConfig,
    /// Whether to enable differential privacy.
    pub enable_privacy: bool,
    /// Model checkpoint interval (rounds).
    pub checkpoint_interval: Option<u64>,
}

impl Default for DistributedTrainingConfig {
    fn default() -> Self {
        Self {
            min_agents: 3,
            max_agents: None,
            max_rounds: Some(100),
            round_timeout_secs: 300,
            aggregation: AggregationConfig::default(),
            enable_privacy: false,
            checkpoint_interval: Some(10),
        }
    }
}

/// A participant in distributed federated learning.
pub struct TrainingParticipant {
    /// Agent ID.
    pub agent_id: String,
    /// Client instance.
    pub client: FederatedClient,
    /// Last active timestamp.
    pub last_active: std::time::Instant,
    /// Samples contributed in current round.
    pub samples: usize,
}

/// Coordinator for distributed federated learning across agents.
pub struct DistributedTrainingCoordinator {
    /// Configuration.
    config: DistributedTrainingConfig,
    /// Server instance.
    server: FederatedServer,
    /// Current participants.
    participants: Arc<RwLock<HashMap<String, TrainingParticipant>>>,
    /// Current round ID.
    current_round: Arc<RwLock<u64>>,
    /// Event sender for notifications.
    event_tx: mpsc::UnboundedSender<DistributedTrainingEvent>,
    /// Model being trained.
    model: Model,
}

impl DistributedTrainingCoordinator {
    /// Create a new distributed training coordinator.
    pub fn new(
        config: DistributedTrainingConfig,
        model: Model,
        event_tx: mpsc::UnboundedSender<DistributedTrainingEvent>,
    ) -> Result<Self, Error> {
        let server = FederatedServer::new(model.clone())?;
        Ok(Self {
            config,
            server,
            participants: Arc::new(RwLock::new(HashMap::new())),
            current_round: Arc::new(RwLock::new(0)),
            event_tx,
            model,
        })
    }

    /// Register a participant agent.
    pub async fn register_participant(
        &self,
        agent_id: String,
        client: FederatedClient,
    ) -> Result<(), Error> {
        let mut participants = self.participants.write().await;
        participants.insert(
            agent_id.clone(),
            TrainingParticipant {
                agent_id,
                client,
                last_active: std::time::Instant::now(),
                samples: 0,
            },
        );
        Ok(())
    }

    /// Unregister a participant.
    pub async fn unregister_participant(&self, agent_id: &str) -> Result<(), Error> {
        let mut participants = self.participants.write().await;
        participants.remove(agent_id);
        Ok(())
    }

    /// Start a new training round if enough participants are available.
    pub async fn start_round(&self) -> Result<bool, Error> {
        let participants = self.participants.read().await;
        if participants.len() < self.config.min_agents {
            return Ok(false);
        }

        let mut current_round = self.current_round.write().await;
        *current_round += 1;
        let round_id = *current_round;

        // Select participants (optionally limit by max_agents)
        let selected: Vec<String> = participants.keys()
            .take(self.config.max_agents.unwrap_or(usize::MAX))
            .cloned()
            .collect();

        drop(participants); // release lock

        // Notify about round start
        let _ = self.event_tx.send(DistributedTrainingEvent::RoundStarted {
            round_id,
            participants: selected.len(),
            model_version: self.model.version,
        });

        // Distribute current model to selected participants
        for agent_id in &selected {
            // In a real implementation, you'd send the model via mesh transport
            // For now, we just update the client's model
            let participants = self.participants.read().await;
            if let Some(participant) = participants.get(agent_id) {
                // participant.client.set_model(self.model.clone())?;
            }
        }

        Ok(true)
    }

    /// Receive an update from a participant.
    pub async fn receive_update(
        &self,
        agent_id: &str,
        update: ClientUpdate,
    ) -> Result<(), Error> {
        // Record the update in the server
        self.server.add_client_update(update.clone())?;

        // Update participant's last active time
        let mut participants = self.participants.write().await;
        if let Some(participant) = participants.get_mut(agent_id) {
            participant.last_active = std::time::Instant::now();
            participant.samples = update.sample_count;
        }

        // Notify about update
        let _ = self.event_tx.send(DistributedTrainingEvent::UpdateReceived {
            client_id: agent_id.to_string(),
            round_id: update.round_id,
            samples: update.sample_count,
        });

        // Check if we have enough updates to aggregate
        let current_updates = self.server.client_updates_count();
        let participants_len = participants.len();
        
        if current_updates >= self.config.min_agents {
            self.aggregate_updates().await?;
        }

        Ok(())
    }

    /// Aggregate received updates and update the global model.
    async fn aggregate_updates(&self) -> Result<(), Error> {
        let round_id = *self.current_round.read().await;
        
        // Perform aggregation
        let aggregated_model = self.server.aggregate()?;
        
        // Update the coordinator's model
        // self.model = aggregated_model; // In real implementation
        
        // Notify about aggregation completion
        let _ = self.event_tx.send(DistributedTrainingEvent::AggregationCompleted {
            round_id,
            aggregated_model_version: aggregated_model.version,
            participant_count: self.server.client_updates_count(),
        });

        // Clear updates for next round
        self.server.clear_updates();

        Ok(())
    }

    /// Get current training statistics.
    pub async fn get_stats(&self) -> DistributedTrainingStats {
        let participants = self.participants.read().await;
        let current_round = *self.current_round.read().await;
        
        DistributedTrainingStats {
            round: current_round,
            active_participants: participants.len(),
            total_updates: self.server.client_updates_count(),
            model_version: self.model.version,
        }
    }

    /// Run continuous training for the specified number of rounds.
    pub async fn run(&self) -> Result<(), Error> {
        let mut rounds_completed = 0;
        
        while self.config.max_rounds.map_or(true, |max| rounds_completed < max) {
            // Wait for enough participants
            while {
                let participants = self.participants.read().await;
                participants.len() < self.config.min_agents
            } {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            // Start a round
            if !self.start_round().await? {
                continue;
            }

            // Wait for round completion or timeout
            let timeout = tokio::time::Duration::from_secs(self.config.round_timeout_secs);
            match tokio::time::timeout(timeout, self.wait_for_round_completion()).await {
                Ok(Ok(())) => {
                    rounds_completed += 1;
                }
                Ok(Err(e)) => {
                    let _ = self.event_tx.send(DistributedTrainingEvent::RoundFailed {
                        round_id: *self.current_round.read().await,
                        reason: e.to_string(),
                    });
                }
                Err(_) => {
                    let _ = self.event_tx.send(DistributedTrainingEvent::RoundFailed {
                        round_id: *self.current_round.read().await,
                        reason: "timeout".to_string(),
                    });
                }
            }

            // Checkpoint if needed
            if let Some(interval) = self.config.checkpoint_interval {
                if rounds_completed % interval == 0 {
                    self.checkpoint().await?;
                }
            }
        }

        Ok(())
    }

    async fn wait_for_round_completion(&self) -> Result<(), Error> {
        // Wait until we have enough updates or all participants have responded
        loop {
            let updates = self.server.client_updates_count();
            let participants = self.participants.read().await.len();
            
            if updates >= self.config.min_agents.min(participants) {
                break;
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        Ok(())
    }

    async fn checkpoint(&self) -> Result<(), Error> {
        // Save model checkpoint
        // In a real implementation, you'd serialize the model to disk
        Ok(())
    }
}

/// Statistics about distributed training.
#[derive(Debug, Clone)]
pub struct DistributedTrainingStats {
    /// Current round number.
    pub round: u64,
    /// Number of active participants.
    pub active_participants: usize,
    /// Total updates received in current round.
    pub total_updates: usize,
    /// Current model version.
    pub model_version: u64,
}

/// Integration with mesh transport for decentralized federated learning.
pub struct MeshFederatedIntegration {
    coordinator: Arc<DistributedTrainingCoordinator>,
}

impl MeshFederatedIntegration {
    /// Create a new mesh integration.
    pub fn new(coordinator: Arc<DistributedTrainingCoordinator>) -> Self {
        Self { coordinator }
    }

    /// Handle incoming mesh message related to federated learning.
    pub async fn handle_message(&self, sender: &str, payload: &[u8]) -> Result<(), Error> {
        // Parse message (in a real implementation, you'd have a proper protocol)
        // For now, just log
        tracing::debug!("Received FL message from {}: {} bytes", sender, payload.len());
        Ok(())
    }

    /// Broadcast model update to all participants.
    pub async fn broadcast_model(&self) -> Result<(), Error> {
        // In a real implementation, you'd use mesh transport to broadcast
        // the model to all registered participants
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let config = DistributedTrainingConfig::default();
        let model = Model {
            name: "test".to_string(),
            layers: vec![],
            parameter_count: 0,
            version: 1,
            metadata: HashMap::new(),
        };
        
        let coordinator = DistributedTrainingCoordinator::new(config, model, tx);
        assert!(coordinator.is_ok());
    }
}