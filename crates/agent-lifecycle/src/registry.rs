//! Registry for managing multiple agents.

use crate::error::{LifecycleError, Result};
use crate::manager::{LifecycleManager, LifecycleManagerConfig};
use common::types::AgentId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Information about a registered agent.
#[derive(Debug, Clone)]
pub struct RegisteredAgent {
    /// Agent ID.
    pub id: AgentId,
    /// Lifecycle manager for the agent.
    pub manager: Arc<LifecycleManager>,
    /// Tags for categorizing agents.
    pub tags: Vec<String>,
    /// When the agent was registered.
    pub registered_at: std::time::SystemTime,
}

/// Registry for managing multiple agents.
pub struct AgentRegistry {
    agents: RwLock<HashMap<AgentId, RegisteredAgent>>,
    next_agent_id: RwLock<AgentId>,
}

impl AgentRegistry {
    /// Create a new agent registry.
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            next_agent_id: RwLock::new(1),
        }
    }

    /// Register a new agent with default configuration.
    pub async fn register_agent(&self, tags: Vec<String>) -> Result<AgentId> {
        let agent_id = {
            let mut next_id = self.next_agent_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        self.register_agent_with_id(agent_id, tags).await?;
        Ok(agent_id)
    }

    /// Register an agent with a specific ID.
    pub async fn register_agent_with_id(&self, agent_id: AgentId, tags: Vec<String>) -> Result<()> {
        let config = LifecycleManagerConfig {
            agent_id,
            ..Default::default()
        };

        let manager = LifecycleManager::new(config)
            .map_err(|e| LifecycleError::Internal(format!("Failed to create manager: {}", e)))?;

        let registered_agent = RegisteredAgent {
            id: agent_id,
            manager: Arc::new(manager),
            tags: tags.clone(),
            registered_at: std::time::SystemTime::now(),
        };

        let mut agents = self.agents.write().await;
        if agents.contains_key(&agent_id) {
            return Err(LifecycleError::Internal(format!(
                "Agent with ID {} already registered",
                agent_id
            )));
        }

        agents.insert(agent_id, registered_agent);
        
        // Update next_agent_id if needed
        let mut next_id = self.next_agent_id.write().await;
        if agent_id >= *next_id {
            *next_id = agent_id + 1;
        }

        info!("Registered agent {} with tags: {:?}", agent_id, tags);
        Ok(())
    }

    /// Unregister an agent.
    pub async fn unregister_agent(&self, agent_id: AgentId) -> Result<()> {
        let mut agents = self.agents.write().await;
        
        if let Some(agent) = agents.remove(&agent_id) {
            // Stop the agent if it's running
            match agent.manager.stop().await {
                Ok(_) => debug!("Agent {} stopped during unregistration", agent_id),
                Err(e) => warn!("Failed to stop agent {} during unregistration: {}", agent_id, e),
            }
            
            info!("Unregistered agent {}", agent_id);
            Ok(())
        } else {
            Err(LifecycleError::AgentNotFound(agent_id))
        }
    }

    /// Get a registered agent.
    pub async fn get_agent(&self, agent_id: AgentId) -> Result<Arc<LifecycleManager>> {
        let agents = self.agents.read().await;
        
        agents
            .get(&agent_id)
            .map(|agent| agent.manager.clone())
            .ok_or_else(|| LifecycleError::AgentNotFound(agent_id))
    }

    /// Get all registered agents.
    pub async fn get_all_agents(&self) -> Vec<RegisteredAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Get agents by tag.
    pub async fn get_agents_by_tag(&self, tag: &str) -> Vec<Arc<LifecycleManager>> {
        let agents = self.agents.read().await;
        
        agents
            .values()
            .filter(|agent| agent.tags.contains(&tag.to_string()))
            .map(|agent| agent.manager.clone())
            .collect()
    }

    /// Initialize all registered agents.
    pub async fn initialize_all(&self) -> Result<()> {
        let agents = self.agents.read().await;
        let mut errors = Vec::new();

        for (agent_id, agent) in agents.iter() {
            match agent.manager.initialize().await {
                Ok(_) => debug!("Initialized agent {}", agent_id),
                Err(e) => {
                    error!("Failed to initialize agent {}: {}", agent_id, e);
                    errors.push((*agent_id, e));
                }
            }
        }

        if errors.is_empty() {
            info!("All agents initialized successfully");
            Ok(())
        } else {
            Err(LifecycleError::Internal(format!(
                "Failed to initialize some agents: {:?}",
                errors
            )))
        }
    }

    /// Start all registered agents.
    pub async fn start_all(&self) -> Result<()> {
        let agents = self.agents.read().await;
        let mut errors = Vec::new();

        for (agent_id, agent) in agents.iter() {
            match agent.manager.start().await {
                Ok(_) => debug!("Started agent {}", agent_id),
                Err(e) => {
                    error!("Failed to start agent {}: {}", agent_id, e);
                    errors.push((*agent_id, e));
                }
            }
        }

        if errors.is_empty() {
            info!("All agents started successfully");
            Ok(())
        } else {
            Err(LifecycleError::Internal(format!(
                "Failed to start some agents: {:?}",
                errors
            )))
        }
    }

    /// Stop all registered agents.
    pub async fn stop_all(&self) -> Result<()> {
        let agents = self.agents.read().await;
        let mut errors = Vec::new();

        for (agent_id, agent) in agents.iter() {
            match agent.manager.stop().await {
                Ok(_) => debug!("Stopped agent {}", agent_id),
                Err(e) => {
                    error!("Failed to stop agent {}: {}", agent_id, e);
                    errors.push((*agent_id, e));
                }
            }
        }

        if errors.is_empty() {
            info!("All agents stopped successfully");
            Ok(())
        } else {
            Err(LifecycleError::Internal(format!(
                "Failed to stop some agents: {:?}",
                errors
            )))
        }
    }

    /// Perform health check on all agents.
    pub async fn check_all_health(&self) -> HashMap<AgentId, crate::health::HealthStatus> {
        let agents = self.agents.read().await;
        let mut results = HashMap::new();

        for (agent_id, agent) in agents.iter() {
            match agent.manager.check_health().await {
                Ok(status) => {
                    results.insert(*agent_id, status);
                    debug!("Health check for agent {}: {}", agent_id, status);
                }
                Err(e) => {
                    error!("Health check failed for agent {}: {}", agent_id, e);
                    results.insert(*agent_id, crate::health::HealthStatus::Unknown);
                }
            }
        }

        results
    }

    /// Get statistics about registered agents.
    pub async fn get_statistics(&self) -> RegistryStatistics {
        let agents = self.agents.read().await;
        
        let mut by_state = HashMap::new();
        let mut by_health = HashMap::new();
        let mut total_tags = 0;
        
        for agent in agents.values() {
            // Get state (async, but we'll do it synchronously for simplicity)
            let state = match agent.manager.get_state().await {
                Ok(state) => state,
                Err(_) => continue,
            };
            
            *by_state.entry(state).or_insert(0) += 1;
            
            // Get health (async)
            let health = match agent.manager.check_health().await {
                Ok(health) => health,
                Err(_) => crate::health::HealthStatus::Unknown,
            };
            
            *by_health.entry(health).or_insert(0) += 1;
            
            total_tags += agent.tags.len();
        }

        RegistryStatistics {
            total_agents: agents.len(),
            by_state,
            by_health,
            total_tags,
            average_tags_per_agent: if agents.is_empty() {
                0.0
            } else {
                total_tags as f64 / agents.len() as f64
            },
        }
    }

    /// Find agents that are in an error state.
    pub async fn find_failed_agents(&self) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        let mut failed = Vec::new();

        for (agent_id, agent) in agents.iter() {
            match agent.manager.get_state().await {
                Ok(state) => {
                    if state == crate::state::AgentState::Failed {
                        failed.push(*agent_id);
                    }
                }
                Err(_) => {
                    // If we can't get the state, consider it failed
                    failed.push(*agent_id);
                }
            }
        }

        failed
    }
}

/// Statistics about the agent registry.
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    /// Total number of registered agents.
    pub total_agents: usize,
    /// Number of agents by state.
    pub by_state: HashMap<crate::state::AgentState, usize>,
    /// Number of agents by health status.
    pub by_health: HashMap<crate::health::HealthStatus, usize>,
    /// Total number of tags across all agents.
    pub total_tags: usize,
    /// Average number of tags per agent.
    pub average_tags_per_agent: f64,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = AgentRegistry::new();
        let agents = registry.get_all_agents().await;
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let registry = AgentRegistry::new();
        
        let agent_id = registry.register_agent(vec!["test".to_string()]).await.unwrap();
        assert_eq!(agent_id, 1);
        
        let agents = registry.get_all_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, 1);
        assert_eq!(agents[0].tags, vec!["test".to_string()]);
    }

    #[tokio::test]
    async fn test_get_agent() {
        let registry = AgentRegistry::new();
        
        let agent_id = registry.register_agent(vec![]).await.unwrap();
        let manager = registry.get_agent(agent_id).await.unwrap();
        
        let state = manager.get_state().await;
        assert_eq!(state, crate::state::AgentState::Initializing);
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let registry = AgentRegistry::new();
        
        let agent_id = registry.register_agent(vec![]).await.unwrap();
        assert_eq!(registry.get_all_agents().await.len(), 1);
        
        registry.unregister_agent(agent_id).await.unwrap();
        assert_eq!(registry.get_all_agents().await.len(), 0);
        
        // Try to get unregistered agent
        let result = registry.get_agent(agent_id).await;
        assert!(matches!(result, Err(LifecycleError::AgentNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_agents_by_tag() {
        let registry = AgentRegistry::new();
        
        registry.register_agent(vec!["worker".to_string()]).await.unwrap();
        registry.register_agent(vec!["coordinator".to_string()]).await.unwrap();
        registry.register_agent(vec!["worker".to_string(), "fast".to_string()]).await.unwrap();
        
        let workers = registry.get_agents_by_tag("worker").await;
        assert_eq!(workers.len(), 2);
        
        let coordinators = registry.get_agents_by_tag("coordinator").await;
        assert_eq!(coordinators.len(), 1);
        
        let fast_agents = registry.get_agents_by_tag("fast").await;
        assert_eq!(fast_agents.len(), 1);
    }
}