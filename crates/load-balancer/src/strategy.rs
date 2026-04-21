//! Load balancing strategies and core balancer implementation.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::{LoadBalancingError, Result};
use crate::metrics::{AgentLoad, LoadMetrics};

/// Load balancing strategy enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoadBalancingStrategy {
    /// Simple round‑robin selection.
    RoundRobin,
    /// Select the agent with the lowest current load.
    LeastLoaded,
    /// Weighted round‑robin based on agent capacity.
    WeightedRoundRobin,
    /// Consistent hashing for sticky sessions.
    ConsistentHashing,
    /// Adaptive strategy that learns from past performance.
    Adaptive,
    /// Predictive strategy using time‑series forecasting.
    Predictive,
}

impl std::fmt::Display for LoadBalancingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoundRobin => write!(f, "RoundRobin"),
            Self::LeastLoaded => write!(f, "LeastLoaded"),
            Self::WeightedRoundRobin => write!(f, "WeightedRoundRobin"),
            Self::ConsistentHashing => write!(f, "ConsistentHashing"),
            Self::Adaptive => write!(f, "Adaptive"),
            Self::Predictive => write!(f, "Predictive"),
        }
    }
}

/// Configuration for the load balancer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadBalancerConfig {
    /// Strategy to use.
    pub strategy: LoadBalancingStrategy,
    /// Update interval for metrics in seconds.
    pub metrics_update_interval_secs: u64,
    /// Whether to enable health checks.
    pub enable_health_checks: bool,
    /// Health check interval in seconds.
    pub health_check_interval_secs: u64,
    /// Maximum number of agents to track.
    pub max_agents: Option<usize>,
    /// Whether to enable automatic agent discovery.
    pub enable_auto_discovery: bool,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::LeastLoaded,
            metrics_update_interval_secs: 5,
            enable_health_checks: true,
            health_check_interval_secs: 30,
            max_agents: Some(100),
            enable_auto_discovery: false,
        }
    }
}

/// Round‑robin load balancing strategy.
pub struct RoundRobinStrategy {
    agents: Vec<String>,
    current_index: usize,
}

impl RoundRobinStrategy {
    /// Create a new round‑robin strategy.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            current_index: 0,
        }
    }

    /// Select the next agent in round‑robin fashion.
    pub fn select(&mut self) -> Result<String> {
        if self.agents.is_empty() {
            return Err(LoadBalancingError::NoAgentsAvailable);
        }
        
        let agent = self.agents[self.current_index].clone();
        self.current_index = (self.current_index + 1) % self.agents.len();
        Ok(agent)
    }

    /// Update the list of agents.
    pub fn update_agents(&mut self, agents: Vec<String>) {
        self.agents = agents;
        if self.current_index >= self.agents.len() && !self.agents.is_empty() {
            self.current_index = 0;
        }
    }
}

/// Least‑loaded load balancing strategy.
pub struct LeastLoadedStrategy {
    agent_loads: HashMap<String, AgentLoad>,
}

impl LeastLoadedStrategy {
    /// Create a new least‑loaded strategy.
    pub fn new() -> Self {
        Self {
            agent_loads: HashMap::new(),
        }
    }

    /// Select the agent with the lowest load.
    pub fn select(&self) -> Result<String> {
        if self.agent_loads.is_empty() {
            return Err(LoadBalancingError::NoAgentsAvailable);
        }

        let (agent, _) = self.agent_loads
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.total_load().partial_cmp(&b.total_load()).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or(LoadBalancingError::NoAgentsAvailable)?;

        Ok(agent.clone())
    }

    /// Update agent load information.
    pub fn update_agent_load(&mut self, agent_id: &str, load: AgentLoad) {
        self.agent_loads.insert(agent_id.to_string(), load);
    }

    /// Remove an agent from consideration.
    pub fn remove_agent(&mut self, agent_id: &str) {
        self.agent_loads.remove(agent_id);
    }
}

/// Weighted round‑robin strategy.
pub struct WeightedRoundRobinStrategy {
    agents: Vec<(String, f64)>, // (agent_id, weight)
    current_weights: Vec<f64>,
    current_index: usize,
}

impl WeightedRoundRobinStrategy {
    /// Create a new weighted round‑robin strategy.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            current_weights: Vec::new(),
            current_index: 0,
        }
    }

    /// Select the next agent using weighted round‑robin.
    pub fn select(&mut self) -> Result<String> {
        if self.agents.is_empty() {
            return Err(LoadBalancingError::NoAgentsAvailable);
        }

        loop {
            self.current_index = (self.current_index + 1) % self.agents.len();
            
            if self.current_index == 0 {
                // Reset weights
                self.current_weights = self.agents.iter().map(|(_, w)| *w).collect();
            }
            
            if self.current_weights[self.current_index] > 0.0 {
                self.current_weights[self.current_index] -= 1.0;
                return Ok(self.agents[self.current_index].0.clone());
            }
        }
    }

    /// Update agents with their weights.
    pub fn update_agents(&mut self, agents: Vec<(String, f64)>) {
        self.agents = agents;
        self.current_weights = self.agents.iter().map(|(_, w)| *w).collect();
        self.current_index = 0;
    }
}

/// Consistent hashing strategy for sticky sessions.
pub struct ConsistentHashingStrategy {
    agents: Vec<String>,
    virtual_nodes: usize,
    ring: HashMap<u32, String>,
}

impl ConsistentHashingStrategy {
    /// Create a new consistent hashing strategy.
    pub fn new(virtual_nodes: usize) -> Self {
        Self {
            agents: Vec::new(),
            virtual_nodes,
            ring: HashMap::new(),
        }
    }

    /// Select an agent for a given key.
    pub fn select(&self, key: &str) -> Result<String> {
        if self.agents.is_empty() {
            return Err(LoadBalancingError::NoAgentsAvailable);
        }

        let hash = Self::hash(key);
        let mut sorted_hashes: Vec<u32> = self.ring.keys().cloned().collect();
        sorted_hashes.sort();

        for ring_hash in sorted_hashes {
            if ring_hash >= hash {
                return Ok(self.ring[&ring_hash].clone());
            }
        }

        // Wrap around to the first
        if let Some(first_hash) = sorted_hashes.first() {
            return Ok(self.ring[first_hash].clone());
        }

        Err(LoadBalancingError::NoAgentsAvailable)
    }

    /// Update the list of agents.
    pub fn update_agents(&mut self, agents: Vec<String>) {
        self.agents = agents;
        self.rebuild_ring();
    }

    fn rebuild_ring(&mut self) {
        self.ring.clear();
        
        for agent in &self.agents {
            for i in 0..self.virtual_nodes {
                let key = format!("{}#{}", agent, i);
                let hash = Self::hash(&key);
                self.ring.insert(hash, agent.clone());
            }
        }
    }

    fn hash(key: &str) -> u32 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as u32
    }
}

/// Main load balancer that wraps strategies.
pub struct LoadBalancer {
    strategy: LoadBalancingStrategy,
    config: LoadBalancerConfig,
    round_robin: RoundRobinStrategy,
    least_loaded: LeastLoadedStrategy,
    weighted_rr: WeightedRoundRobinStrategy,
    consistent_hashing: ConsistentHashingStrategy,
    agent_loads: HashMap<String, AgentLoad>,
    agent_weights: HashMap<String, f64>,
}

impl LoadBalancer {
    /// Create a new load balancer.
    pub fn new(strategy: LoadBalancingStrategy, config: LoadBalancerConfig) -> Self {
        Self {
            strategy,
            config,
            round_robin: RoundRobinStrategy::new(),
            least_loaded: LeastLoadedStrategy::new(),
            weighted_rr: WeightedRoundRobinStrategy::new(),
            consistent_hashing: ConsistentHashingStrategy::new(100), // 100 virtual nodes
            agent_loads: HashMap::new(),
            agent_weights: HashMap::new(),
        }
    }

    /// Register an agent with initial load.
    pub async fn register_agent(&mut self, agent_id: &str, initial_load: AgentLoad) -> Result<()> {
        self.agent_loads.insert(agent_id.to_string(), initial_load.clone());
        self.agent_weights.insert(agent_id.to_string(), 1.0); // Default weight
        
        // Update all strategies
        self.update_strategies();
        
        Ok(())
    }

    /// Unregister an agent.
    pub async fn unregister_agent(&mut self, agent_id: &str) -> Result<()> {
        self.agent_loads.remove(agent_id);
        self.agent_weights.remove(agent_id);
        
        self.update_strategies();
        
        Ok(())
    }

    /// Update an agent's load metrics.
    pub async fn update_agent_load(&mut self, agent_id: &str, load: AgentLoad) -> Result<()> {
        if !self.agent_loads.contains_key(agent_id) {
            return Err(LoadBalancingError::AgentNotFound(agent_id.to_string()));
        }
        
        self.agent_loads.insert(agent_id.to_string(), load);
        self.least_loaded.update_agent_load(agent_id, self.agent_loads[agent_id].clone());
        
        Ok(())
    }

    /// Update an agent's weight (for weighted strategies).
    pub async fn update_agent_weight(&mut self, agent_id: &str, weight: f64) -> Result<()> {
        if !self.agent_weights.contains_key(agent_id) {
            return Err(LoadBalancingError::AgentNotFound(agent_id.to_string()));
        }
        
        self.agent_weights.insert(agent_id.to_string(), weight);
        self.update_strategies();
        
        Ok(())
    }

    /// Select an agent using the configured strategy.
    pub async fn select_agent(&mut self) -> Result<String> {
        self.select_agent_with_key(None).await
    }

    /// Select an agent with a key (for consistent hashing).
    pub async fn select_agent_with_key(&mut self, key: Option<&str>) -> Result<String> {
        match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.round_robin.select(),
            LoadBalancingStrategy::LeastLoaded => self.least_loaded.select(),
            LoadBalancingStrategy::WeightedRoundRobin => self.weighted_rr.select(),
            LoadBalancingStrategy::ConsistentHashing => {
                let key = key.ok_or_else(|| LoadBalancingError::InvalidConfig("Key required for consistent hashing".to_string()))?;
                self.consistent_hashing.select(key)
            }
            LoadBalancingStrategy::Adaptive => {
                // Fall back to least loaded for now
                // In practice, this would use the adaptive module
                self.least_loaded.select()
            }
            LoadBalancingStrategy::Predictive => {
                // Fall back to least loaded for now
                // In practice, this would use the predictive module
                self.least_loaded.select()
            }
        }
    }

    /// Get the current strategy.
    pub fn strategy(&self) -> LoadBalancingStrategy {
        self.strategy
    }

    /// Change the load balancing strategy.
    pub fn set_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
    }

    /// Get agent load information.
    pub fn get_agent_load(&self, agent_id: &str) -> Option<&AgentLoad> {
        self.agent_loads.get(agent_id)
    }

    /// Get all agent loads.
    pub fn get_all_agent_loads(&self) -> &HashMap<String, AgentLoad> {
        &self.agent_loads
    }

    fn update_strategies(&mut self) {
        let agents: Vec<String> = self.agent_loads.keys().cloned().collect();
        self.round_robin.update_agents(agents.clone());
        
        let weighted_agents: Vec<(String, f64)> = agents
            .iter()
            .map(|id| (id.clone(), *self.agent_weights.get(id).unwrap_or(&1.0)))
            .collect();
        self.weighted_rr.update_agents(weighted_agents);
        
        self.consistent_hashing.update_agents(agents);
        
        // Update least loaded with all current loads
        for (id, load) in &self.agent_loads {
            self.least_loaded.update_agent_load(id, load.clone());
        }
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(LoadBalancingStrategy::LeastLoaded, LoadBalancerConfig::default())
    }
}