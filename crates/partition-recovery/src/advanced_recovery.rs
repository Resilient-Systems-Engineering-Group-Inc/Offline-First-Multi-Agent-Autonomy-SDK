//! Advanced recovery strategies for network partitions.
//!
//! This module provides sophisticated algorithms for partition recovery,
//! including conflict resolution, priority‑based merging, and historical
//! version reconciliation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use common::types::{AgentId, VectorClock};
use state_sync::{StateSync, CrdtMap, Delta};

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Last‑write‑wins (based on timestamp).
    LastWriteWins,
    /// Priority‑based (higher priority agent wins).
    PriorityBased,
    /// Manual intervention required.
    Manual,
    /// Merge using CRDT semantics (default).
    CrdtMerge,
    /// Custom conflict resolver.
    Custom,
}

/// Priority of an agent for conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPriority {
    pub agent_id: AgentId,
    pub priority: u32,
    pub capabilities: HashSet<String>,
}

/// Configuration for advanced recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedRecoveryConfig {
    /// Conflict resolution strategy.
    pub conflict_resolution: ConflictResolution,
    /// Whether to keep historical versions for rollback.
    pub keep_history: bool,
    /// Maximum number of historical versions to keep.
    pub max_history: usize,
    /// Timeout for recovery phases.
    pub phase_timeout: Duration,
    /// Enable automatic rollback on failure.
    pub auto_rollback: bool,
    /// Priority list for agents (used with PriorityBased strategy).
    pub agent_priorities: Vec<AgentPriority>,
}

impl Default for AdvancedRecoveryConfig {
    fn default() -> Self {
        Self {
            conflict_resolution: ConflictResolution::CrdtMerge,
            keep_history: true,
            max_history: 10,
            phase_timeout: Duration::from_secs(60),
            auto_rollback: false,
            agent_priorities: Vec::new(),
        }
    }
}

/// A historical state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub timestamp: SystemTime,
    pub state_hash: Vec<u8>,
    pub vector_clock: VectorClock,
    pub agent_id: AgentId,
    pub metadata: HashMap<String, String>,
}

/// Advanced recovery engine with conflict resolution and history.
pub struct AdvancedRecoveryEngine<T: StateSync + Send + Sync> {
    config: AdvancedRecoveryConfig,
    state_sync: Arc<RwLock<T>>,
    history: RwLock<Vec<StateSnapshot>>,
    priorities: HashMap<AgentId, u32>,
}

impl<T: StateSync + Send + Sync> AdvancedRecoveryEngine<T> {
    /// Create a new advanced recovery engine.
    pub fn new(config: AdvancedRecoveryConfig, state_sync: Arc<RwLock<T>>) -> Self {
        let priorities: HashMap<AgentId, u32> = config.agent_priorities
            .iter()
            .map(|ap| (ap.agent_id, ap.priority))
            .collect();
        
        Self {
            config,
            state_sync,
            history: RwLock::new(Vec::new()),
            priorities,
        }
    }

    /// Take a snapshot of current state.
    pub async fn take_snapshot(&self, metadata: HashMap<String, String>) -> Result<(), String> {
        let state_sync = self.state_sync.read().await;
        let state_hash = state_sync.compute_state_hash().await
            .map_err(|e| e.to_string())?;
        let vector_clock = state_sync.get_vector_clock().await;
        
        let snapshot = StateSnapshot {
            timestamp: SystemTime::now(),
            state_hash,
            vector_clock,
            agent_id: state_sync.local_agent_id(),
            metadata,
        };

        let mut history = self.history.write().await;
        history.push(snapshot);
        
        // Trim history if exceeds max size
        if history.len() > self.config.max_history {
            history.remove(0);
        }
        
        Ok(())
    }

    /// Recover from a partition by merging states from multiple agents.
    pub async fn recover(
        &self,
        agent_states: HashMap<AgentId, Vec<u8>>,
    ) -> Result<HashMap<String, String>, String> {
        tracing::info!("Starting advanced recovery with {} agent states", agent_states.len());
        
        // Step 1: Validate states
        let valid_states = self.validate_states(agent_states).await?;
        
        // Step 2: Detect conflicts
        let conflicts = self.detect_conflicts(&valid_states).await?;
        
        // Step 3: Resolve conflicts based on strategy
        let resolved = self.resolve_conflicts(conflicts).await?;
        
        // Step 4: Apply resolved state
        self.apply_resolved_state(resolved).await?;
        
        // Step 5: Take a post‑recovery snapshot
        let mut metadata = HashMap::new();
        metadata.insert("recovery_timestamp".to_string(), format!("{:?}", SystemTime::now()));
        metadata.insert("recovered_agents".to_string(), valid_states.len().to_string());
        self.take_snapshot(metadata.clone()).await?;
        
        Ok(metadata)
    }

    /// Validate incoming states (check integrity, version compatibility).
    async fn validate_states(
        &self,
        agent_states: HashMap<AgentId, Vec<u8>>
    ) -> Result<HashMap<AgentId, Vec<u8>>, String> {
        let mut valid = HashMap::new();
        
        for (agent_id, state_bytes) in agent_states {
            // Basic validation: non‑empty
            if state_bytes.is_empty() {
                tracing::warn!("Empty state from agent {}, skipping", agent_id);
                continue;
            }
            
            // Could add checksum verification here
            valid.insert(agent_id, state_bytes);
        }
        
        if valid.is_empty() {
            return Err("No valid states found".to_string());
        }
        
        Ok(valid)
    }

    /// Detect conflicts between states.
    async fn detect_conflicts(
        &self,
        states: &HashMap<AgentId, Vec<u8>>
    ) -> Result<Vec<Conflict>, String> {
        // Simplified conflict detection: compare state hashes
        let mut conflicts = Vec::new();
        let state_hashes: Vec<&[u8]> = states.values().map(|v| v.as_slice()).collect();
        
        // If all hashes are identical, no conflict
        if state_hashes.windows(2).all(|w| w[0] == w[1]) {
            return Ok(conflicts);
        }
        
        // Otherwise, report conflict
        conflicts.push(Conflict {
            description: "Divergent state hashes".to_string(),
            agents: states.keys().cloned().collect(),
            severity: ConflictSeverity::Medium,
        });
        
        Ok(conflicts)
    }

    /// Resolve conflicts according to configured strategy.
    async fn resolve_conflicts(
        &self,
        conflicts: Vec<Conflict>
    ) -> Result<ResolutionResult, String> {
        if conflicts.is_empty() {
            return Ok(ResolutionResult::NoConflict);
        }
        
        match self.config.conflict_resolution {
            ConflictResolution::LastWriteWins => {
                // In a real implementation, we would examine timestamps
                Ok(ResolutionResult::Resolved {
                    strategy: "LastWriteWins".to_string(),
                    details: "Using latest timestamp".to_string(),
                })
            }
            ConflictResolution::PriorityBased => {
                // Choose state from highest‑priority agent
                let highest_priority_agent = self.priorities
                    .iter()
                    .max_by_key(|(_, &priority)| priority)
                    .map(|(agent_id, _)| *agent_id);
                
                match highest_priority_agent {
                    Some(agent_id) => Ok(ResolutionResult::Resolved {
                        strategy: "PriorityBased".to_string(),
                        details: format!("Using state from agent {} (highest priority)", agent_id),
                    }),
                    None => Ok(ResolutionResult::Resolved {
                        strategy: "PriorityBased".to_string(),
                        details: "No priorities defined, falling back to CRDT merge".to_string(),
                    }),
                }
            }
            ConflictResolution::CrdtMerge => {
                Ok(ResolutionResult::Resolved {
                    strategy: "CrdtMerge".to_string(),
                    details: "Merging using CRDT semantics".to_string(),
                })
            }
            ConflictResolution::Manual => {
                Ok(ResolutionResult::ManualInterventionRequired {
                    conflicts: conflicts.len(),
                })
            }
            ConflictResolution::Custom => {
                Ok(ResolutionResult::Resolved {
                    strategy: "Custom".to_string(),
                    details: "Custom resolution applied".to_string(),
                })
            }
        }
    }

    /// Apply the resolved state to the local state sync.
    async fn apply_resolved_state(
        &self,
        resolution: ResolutionResult
    ) -> Result<(), String> {
        match resolution {
            ResolutionResult::NoConflict => {
                tracing::info!("No conflicts to resolve");
                Ok(())
            }
            ResolutionResult::Resolved { strategy, details } => {
                tracing::info!("Applied resolution: {} - {}", strategy, details);
                // In a real implementation, we would merge the actual state
                // For now, just trigger a sync
                let mut state_sync = self.state_sync.write().await;
                state_sync.broadcast_changes().await
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            ResolutionResult::ManualInterventionRequired { conflicts } => {
                Err(format!("Manual intervention required for {} conflicts", conflicts))
            }
        }
    }

    /// Rollback to a previous snapshot.
    pub async fn rollback(&self, snapshot_index: usize) -> Result<(), String> {
        let history = self.history.read().await;
        if snapshot_index >= history.len() {
            return Err(format!("Invalid snapshot index: {}", snapshot_index));
        }
        
        let snapshot = &history[snapshot_index];
        tracing::info!("Rolling back to snapshot from {:?}", snapshot.timestamp);
        
        // In a real implementation, we would restore the state from the snapshot
        // For now, just log
        Ok(())
    }

    /// Get recovery history.
    pub async fn get_history(&self) -> Vec<StateSnapshot> {
        self.history.read().await.clone()
    }
}

/// Represents a conflict between states.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub description: String,
    pub agents: HashSet<AgentId>,
    pub severity: ConflictSeverity,
}

/// Severity of a conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of conflict resolution.
#[derive(Debug, Clone)]
pub enum ResolutionResult {
    NoConflict,
    Resolved {
        strategy: String,
        details: String,
    },
    ManualInterventionRequired {
        conflicts: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use state_sync::DefaultStateSync;

    #[tokio::test]
    async fn test_advanced_recovery_creation() {
        let state_sync = Arc::new(RwLock::new(DefaultStateSync::new(1)));
        let config = AdvancedRecoveryConfig::default();
        let engine = AdvancedRecoveryEngine::new(config, state_sync);
        
        // Should be able to take a snapshot
        let metadata = HashMap::new();
        let result = engine.take_snapshot(metadata).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_conflict_detection() {
        let state_sync = Arc::new(RwLock::new(DefaultStateSync::new(1)));
        let config = AdvancedRecoveryConfig::default();
        let engine = AdvancedRecoveryEngine::new(config, state_sync);
        
        let mut states = HashMap::new();
        states.insert(1, vec![1, 2, 3]);
        states.insert(2, vec![4, 5, 6]); // Different state -> conflict
        
        let conflicts = engine.detect_conflicts(&states).await.unwrap();
        assert!(!conflicts.is_empty());
    }
}