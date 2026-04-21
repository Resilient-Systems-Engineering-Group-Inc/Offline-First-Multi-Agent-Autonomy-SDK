//! Distributed load balancing coordination.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::sync::{RwLock, mpsc};
use tokio::time::interval;
use crate::error::{LoadBalancingError, Result};
use crate::metrics::{AgentLoad, LoadMetrics, LoadMetricsCollector};
use crate::strategy::{LoadBalancer, LoadBalancerConfig, LoadBalancingStrategy};

/// Message types for distributed coordination.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CoordinationMessage {
    /// Heartbeat from an agent.
    Heartbeat {
        agent_id: String,
        load: AgentLoad,
        timestamp: SystemTime,
    },
    /// Load update from an agent.
    LoadUpdate {
        agent_id: String,
        load: AgentLoad,
    },
    /// Request for load balancing decision.
    BalanceRequest {
        request_id: String,
        task_complexity: f64,
        requirements: HashMap<String, f64>,
    },
    /// Load balancing decision.
    BalanceDecision {
        request_id: String,
        selected_agent: String,
        predicted_load: f64,
        confidence: f64,
    },
    /// Agent registration.
    AgentRegistration {
        agent_id: String,
        capabilities: HashMap<String, f64>,
    },
    /// Agent unregistration.
    AgentUnregistration {
        agent_id: String,
    },
    /// Sync request for global state.
    SyncRequest {
        requester_id: String,
    },
    /// Sync response with global state.
    SyncResponse {
        global_metrics: LoadMetrics,
        timestamp: SystemTime,
    },
}

/// Distributed coordinator configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DistributedCoordinatorConfig {
    /// Heartbeat interval in seconds.
    pub heartbeat_interval_secs: u64,
    /// Heartbeat timeout in seconds.
    pub heartbeat_timeout_secs: u64,
    /// Sync interval in seconds.
    pub sync_interval_secs: u64,
    /// Election timeout in seconds.
    pub election_timeout_secs: u64,
    /// Whether to enable leader election.
    pub enable_leader_election: bool,
    /// Quorum size (minimum agents for decisions).
    pub quorum_size: usize,
    /// Maximum agents per coordinator.
    pub max_agents: usize,
}

impl Default for DistributedCoordinatorConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: 5,
            heartbeat_timeout_secs: 30,
            sync_interval_secs: 10,
            election_timeout_secs: 10,
            enable_leader_election: true,
            quorum_size: 1,
            max_agents: 100,
        }
    }
}

/// Agent information in distributed system.
#[derive(Debug, Clone)]
struct DistributedAgentInfo {
    /// Agent ID.
    agent_id: String,
    /// Current load.
    load: AgentLoad,
    /// Capabilities.
    capabilities: HashMap<String, f64>,
    /// Last heartbeat time.
    last_heartbeat: SystemTime,
    /// Whether agent is active.
    active: bool,
    /// Coordinator that owns this agent.
    coordinator_id: Option<String>,
}

impl DistributedAgentInfo {
    fn new(agent_id: &str, load: AgentLoad, capabilities: HashMap<String, f64>) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            load,
            capabilities,
            last_heartbeat: SystemTime::now(),
            active: true,
            coordinator_id: None,
        }
    }
    
    fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
        self.active = true;
    }
    
    fn is_timed_out(&self, timeout_secs: u64) -> bool {
        SystemTime::now()
            .duration_since(self.last_heartbeat)
            .unwrap_or_default()
            .as_secs() > timeout_secs
    }
}

/// Distributed load balancer coordinator.
pub struct DistributedCoordinator {
    /// Coordinator ID.
    id: String,
    /// Configuration.
    config: DistributedCoordinatorConfig,
    /// Local load balancer.
    local_balancer: Arc<RwLock<LoadBalancer>>,
    /// Registered agents.
    agents: Arc<RwLock<HashMap<String, DistributedAgentInfo>>>,
    /// Message receiver.
    message_rx: Option<mpsc::UnboundedReceiver<CoordinationMessage>>,
    /// Message sender.
    message_tx: mpsc::UnboundedSender<CoordinationMessage>,
    /// Whether coordinator is leader.
    is_leader: bool,
    /// Leader ID (if not self).
    leader_id: Option<String>,
    /// Active tasks.
    active_tasks: Arc<RwLock<HashMap<String, String>>>, // task_id -> agent_id
}

impl DistributedCoordinator {
    /// Create a new distributed coordinator.
    pub fn new(
        id: &str,
        config: DistributedCoordinatorConfig,
        balancer_config: LoadBalancerConfig,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        Self {
            id: id.to_string(),
            config: config.clone(),
            local_balancer: Arc::new(RwLock::new(LoadBalancer::new(
                LoadBalancingStrategy::LeastLoaded,
                balancer_config,
            ))),
            agents: Arc::new(RwLock::new(HashMap::new())),
            message_rx: Some(message_rx),
            message_tx,
            is_leader: false,
            leader_id: None,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start the coordinator.
    pub async fn start(&mut self) -> Result<()> {
        let mut message_rx = self.message_rx.take().unwrap();
        let agents = self.agents.clone();
        let config = self.config.clone();
        let id = self.id.clone();
        
        // Start heartbeat monitoring task
        let agents_clone = agents.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config_clone.heartbeat_interval_secs));
            loop {
                interval.tick().await;
                
                let mut agents = agents_clone.write().await;
                let now = SystemTime::now();
                
                // Check for timed out agents
                let timed_out: Vec<String> = agents
                    .iter()
                    .filter(|(_, info)| info.is_timed_out(config_clone.heartbeat_timeout_secs))
                    .map(|(id, _)| id.clone())
                    .collect();
                
                for agent_id in timed_out {
                    agents.remove(&agent_id);
                    tracing::info!("Agent {} timed out", agent_id);
                }
            }
        });
        
        // Start message processing task
        let local_balancer = self.local_balancer.clone();
        let agents_clone = agents.clone();
        let active_tasks = self.active_tasks.clone();
        let message_tx = self.message_tx.clone();
        
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                match message {
                    CoordinationMessage::Heartbeat { agent_id, load, timestamp } => {
                        let mut agents = agents_clone.write().await;
                        if let Some(info) = agents.get_mut(&agent_id) {
                            info.update_heartbeat();
                            info.load = load;
                        } else {
                            // New agent
                            agents.insert(
                                agent_id.clone(),
                                DistributedAgentInfo::new(&agent_id, load, HashMap::new()),
                            );
                            
                            // Register with local balancer
                            let mut balancer = local_balancer.write().await;
                            if let Err(e) = balancer.register_agent(&agent_id, load).await {
                                tracing::error!("Failed to register agent {}: {}", agent_id, e);
                            }
                        }
                    }
                    
                    CoordinationMessage::LoadUpdate { agent_id, load } => {
                        let mut agents = agents_clone.write().await;
                        if let Some(info) = agents.get_mut(&agent_id) {
                            info.load = load.clone();
                        }
                        
                        // Update local balancer
                        let mut balancer = local_balancer.write().await;
                        if let Err(e) = balancer.update_agent_load(&agent_id, load).await {
                            tracing::error!("Failed to update agent load {}: {}", agent_id, e);
                        }
                    }
                    
                    CoordinationMessage::BalanceRequest { request_id, task_complexity, requirements } => {
                        // Select agent using local balancer
                        let mut balancer = local_balancer.write().await;
                        match balancer.select_agent().await {
                            Ok(selected_agent) => {
                                // Record task assignment
                                let mut tasks = active_tasks.write().await;
                                tasks.insert(request_id.clone(), selected_agent.clone());
                                
                                // Send decision
                                let _ = message_tx.send(CoordinationMessage::BalanceDecision {
                                    request_id,
                                    selected_agent,
                                    predicted_load: 0.5, // Placeholder
                                    confidence: 0.8,     // Placeholder
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to select agent: {}", e);
                            }
                        }
                    }
                    
                    CoordinationMessage::AgentRegistration { agent_id, capabilities } => {
                        let mut agents = agents_clone.write().await;
                        if !agents.contains_key(&agent_id) {
                            agents.insert(
                                agent_id.clone(),
                                DistributedAgentInfo::new(
                                    &agent_id,
                                    AgentLoad::default(),
                                    capabilities,
                                ),
                            );
                            tracing::info!("Registered new agent: {}", agent_id);
                        }
                    }
                    
                    CoordinationMessage::AgentUnregistration { agent_id } => {
                        let mut agents = agents_clone.write().await;
                        agents.remove(&agent_id);
                        
                        // Unregister from local balancer
                        let mut balancer = local_balancer.write().await;
                        let _ = balancer.unregister_agent(&agent_id).await;
                        
                        tracing::info!("Unregistered agent: {}", agent_id);
                    }
                    
                    _ => {
                        // Handle other message types
                    }
                }
            }
        });
        
        tracing::info!("Distributed coordinator {} started", self.id);
        Ok(())
    }
    
    /// Register a local agent.
    pub async fn register_agent(
        &self,
        agent_id: &str,
        capabilities: HashMap<String, f64>,
    ) -> Result<()> {
        let mut agents = self.agents.write().await;
        
        if agents.contains_key(agent_id) {
            return Err(LoadBalancingError::Other(format!("Agent {} already registered", agent_id)));
        }
        
        agents.insert(
            agent_id.to_string(),
            DistributedAgentInfo::new(agent_id, AgentLoad::default(), capabilities),
        );
        
        // Register with local balancer
        let mut balancer = self.local_balancer.write().await;
        balancer.register_agent(agent_id, AgentLoad::default()).await?;
        
        // Broadcast registration
        let _ = self.message_tx.send(CoordinationMessage::AgentRegistration {
            agent_id: agent_id.to_string(),
            capabilities,
        });
        
        Ok(())
    }
    
    /// Unregister a local agent.
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        
        if !agents.contains_key(agent_id) {
            return Err(LoadBalancingError::AgentNotFound(agent_id.to_string()));
        }
        
        agents.remove(agent_id);
        
        // Unregister from local balancer
        let mut balancer = self.local_balancer.write().await;
        balancer.unregister_agent(agent_id).await?;
        
        // Broadcast unregistration
        let _ = self.message_tx.send(CoordinationMessage::AgentUnregistration {
            agent_id: agent_id.to_string(),
        });
        
        Ok(())
    }
    
    /// Send heartbeat for local agent.
    pub async fn send_heartbeat(&self, agent_id: &str, load: AgentLoad) -> Result<()> {
        let _ = self.message_tx.send(CoordinationMessage::Heartbeat {
            agent_id: agent_id.to_string(),
            load,
            timestamp: SystemTime::now(),
        });
        
        Ok(())
    }
    
    /// Request load balancing decision.
    pub async fn request_balance(
        &self,
        task_complexity: f64,
        requirements: HashMap<String, f64>,
    ) -> Result<String> {
        let request_id = uuid::Uuid::new_v4().to_string();
        
        let _ = self.message_tx.send(CoordinationMessage::BalanceRequest {
            request_id: request_id.clone(),
            task_complexity,
            requirements,
        });
        
        // Wait for decision (simplified - in practice would use request-response)
        // For now, use local balancer directly
        let balancer = self.local_balancer.read().await;
        balancer.select_agent().await
    }
    
    /// Get agent count.
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }
    
    /// Get active agent count.
    pub async fn active_agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.values().filter(|info| info.active).count()
    }
    
    /// Get global load metrics.
    pub async fn get_global_metrics(&self) -> LoadMetrics {
        let agents = self.agents.read().await;
        let mut metrics = LoadMetrics::new();
        
        for (agent_id, info) in agents.iter() {
            if info.active {
                metrics.update_agent(agent_id, info.load.clone());
            }
        }
        
        metrics
    }
    
    /// Check if quorum is reached.
    pub async fn has_quorum(&self) -> bool {
        self.active_agent_count().await >= self.config.quorum_size
    }
    
    /// Get message sender for external use.
    pub fn message_sender(&self) -> mpsc::UnboundedSender<CoordinationMessage> {
        self.message_tx.clone()
    }
}

/// Distributed load balancer wrapper.
pub struct DistributedLoadBalancer {
    /// Coordinator.
    coordinator: Arc<DistributedCoordinator>,
    /// Local agent ID (if this node has an agent).
    local_agent_id: Option<String>,
}

impl DistributedLoadBalancer {
    /// Create a new distributed load balancer.
    pub async fn new(
        coordinator_id: &str,
        coordinator_config: DistributedCoordinatorConfig,
        balancer_config: LoadBalancerConfig,
    ) -> Result<Self> {
        let coordinator = DistributedCoordinator::new(
            coordinator_id,
            coordinator_config,
            balancer_config,
        );
        
        Ok(Self {
            coordinator: Arc::new(coordinator),
            local_agent_id: None,
        })
    }
    
    /// Start the distributed load balancer.
    pub async fn start(&mut self) -> Result<()> {
        let mut coordinator = Arc::get_mut(&mut self.coordinator)
            .ok_or_else(|| LoadBalancingError::Other("Cannot get mutable reference to coordinator".to_string()))?;
        
        coordinator.start().await?;
        Ok(())
    }
    
    /// Register this node as an agent.
    pub async fn register_as_agent(
        &mut self,
        agent_id: &str,
        capabilities: HashMap<String, f64>,
    ) -> Result<()> {
        self.coordinator.register_agent(agent_id, capabilities).await?;
        self.local_agent_id = Some(agent_id.to_string());
        Ok(())
    }
    
    /// Update local agent load.
    pub async fn update_local_load(&self, load: AgentLoad) -> Result<()> {
        let agent_id = self.local_agent_id
            .as_ref()
            .ok_or_else(|| LoadBalancingError::Other("No local agent registered".to_string()))?;
        
        self.coordinator.send_heartbeat(agent_id, load).await
    }
    
    /// Select an agent for a task.
    pub async fn select_agent(
        &self,
        task_complexity: f64,
        requirements: HashMap<String, f64>,
    ) -> Result<String> {
        self.coordinator.request_balance(task_complexity, requirements).await
    }
    
    /// Get coordinator.
    pub fn coordinator(&self) -> &DistributedCoordinator {
        &self.coordinator
    }
    
    /// Get agent count.
    pub async fn agent_count(&self) -> usize {
        self.coordinator.agent_count().await
    }
    
    /// Check if system has quorum.
    pub async fn has_quorum(&self) -> bool {
        self.coordinator.has_quorum().await
    }
}