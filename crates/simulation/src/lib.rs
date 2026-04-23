//! Simulation environment for the Multi-Agent SDK.
//!
//! Provides:
//! - Multi-agent simulation
//! - Physics engine integration (Gazebo, Isaac Sim)
//! - ROS2/Gazebo bridge
//! - Scenario testing
//! - Performance benchmarking

pub mod world;
pub mod agent;
pub mod scenario;
pub mod physics;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::info;

pub use world::*;
pub use agent::*;
pub use scenario::*;
pub use physics::*;

/// Simulation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub engine: SimulationEngine,
    pub world_file: PathBuf,
    pub time_step: f64,
    pub real_time_factor: f64,
    pub enable_physics: bool,
    pub enable_collisions: bool,
    pub max_agents: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationEngine {
    Gazebo,
    IsaacSim,
    Unity,
    Custom(String),
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            engine: SimulationEngine::Gazebo,
            world_file: PathBuf::from("./worlds/default.world"),
            time_step: 0.01,
            real_time_factor: 1.0,
            enable_physics: true,
            enable_collisions: true,
            max_agents: 100,
        }
    }
}

/// Simulation manager.
pub struct SimulationManager {
    config: SimulationConfig,
    world: RwLock<Option<SimulationWorld>>,
    agents: RwLock<Vec<SimulatedAgent>>,
    running: RwLock<bool>,
}

impl SimulationManager {
    /// Create new simulation manager.
    pub fn new(config: SimulationConfig) -> Self {
        Self {
            config,
            world: RwLock::new(None),
            agents: RwLock::new(vec![]),
            running: RwLock::new(false),
        }
    }

    /// Initialize simulation.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing simulation with engine: {:?}", self.config.engine);

        // Create world
        let mut world = SimulationWorld::new(&self.config)?;
        world.load(&self.config.world_file).await?;

        *self.world.write().await = Some(world);
        
        info!("Simulation initialized");
        Ok(())
    }

    /// Start simulation.
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;

        info!("Simulation started");
        Ok(())
    }

    /// Stop simulation.
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;

        info!("Simulation stopped");
        Ok(())
    }

    /// Add agent to simulation.
    pub async fn add_agent(&self, config: AgentConfig) -> Result<String> {
        if self.agents.read().await.len() >= self.config.max_agents {
            return Err(anyhow::anyhow!("Maximum number of agents reached"));
        }

        let agent = SimulatedAgent::new(&config, &self.config.engine)?;
        let agent_id = agent.id().to_string();

        // Spawn agent in world
        if let Some(world) = self.world.write().await.as_mut() {
            world.spawn_agent(&agent).await?;
        }

        let mut agents = self.agents.write().await;
        agents.push(agent);

        info!("Agent added: {}", agent_id);
        Ok(agent_id)
    }

    /// Remove agent from simulation.
    pub async fn remove_agent(&self, agent_id: &str) -> Result<()> {
        // Despawn from world
        if let Some(world) = self.world.write().await.as_mut() {
            world.despawn_agent(agent_id).await?;
        }

        let mut agents = self.agents.write().await;
        agents.retain(|a| a.id() != agent_id);

        info!("Agent removed: {}", agent_id);
        Ok(())
    }

    /// Get all agents.
    pub async fn get_agents(&self) -> Vec<SimulatedAgent> {
        let agents = self.agents.read().await;
        agents.clone()
    }

    /// Get agent state.
    pub async fn get_agent_state(&self, agent_id: &str) -> Result<AgentState> {
        let agents = self.agents.read().await;
        
        let agent = agents.iter()
            .find(|a| a.id() == agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;

        Ok(agent.get_state())
    }

    /// Run simulation step.
    pub async fn step(&self) -> Result<SimulationStep> {
        let world = self.world.read().await;
        let agents = self.agents.read().await;

        if world.is_none() {
            return Err(anyhow::anyhow!("Simulation not initialized"));
        }

        let world = world.as_ref().unwrap();
        
        // Update world physics
        let physics_state = world.step(self.config.time_step).await?;

        // Update agent states
        let mut agent_states = vec![];
        for agent in agents.iter() {
            let state = agent.update(&physics_state, self.config.time_step)?;
            agent_states.push(state);
        }

        Ok(SimulationStep {
            timestamp: chrono::Utc::now(),
            time_step: self.config.time_step,
            physics_state,
            agent_states,
        })
    }

    /// Run simulation for duration.
    pub async fn run_for(&self, duration_secs: f64) -> Result<Vec<SimulationStep>> {
        let mut steps = vec![];
        let mut elapsed = 0.0;

        while elapsed < duration_secs {
            let step = self.step().await?;
            steps.push(step);
            elapsed += self.config.time_step;
        }

        Ok(steps)
    }

    /// Load scenario.
    pub async fn load_scenario(&self, scenario: &Scenario) -> Result<()> {
        if let Some(world) = self.world.write().await.as_mut() {
            world.load_scenario(scenario).await?;
        }

        info!("Scenario loaded: {}", scenario.name);
        Ok(())
    }

    /// Get simulation statistics.
    pub async fn get_stats(&self) -> SimulationStats {
        let agents = self.agents.read().await;
        let running = *self.running.read().await;

        SimulationStats {
            engine: format!("{:?}", self.config.engine),
            world_file: self.config.world_file.to_string_lossy().to_string(),
            total_agents: agents.len() as i64,
            running,
            time_step: self.config.time_step,
            real_time_factor: self.config.real_time_factor,
        }
    }
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub initial_position: [f64; 3],
    pub initial_orientation: [f64; 4], // quaternion
    pub model: String,
    pub capabilities: Vec<String>,
}

/// Simulated agent.
pub struct SimulatedAgent {
    id: String,
    name: String,
    state: AgentState,
    model: String,
    capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub position: [f64; 3],
    pub orientation: [f64; 4],
    pub velocity: [f64; 3],
    pub angular_velocity: [f64; 3],
    pub battery_level: f64,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Failed,
    Stopped,
}

impl SimulatedAgent {
    pub fn new(config: &AgentConfig, engine: &SimulationEngine) -> Result<Self> {
        Ok(Self {
            id: config.id.clone(),
            name: config.name.clone(),
            state: AgentState {
                position: config.initial_position,
                orientation: config.initial_orientation,
                velocity: [0.0, 0.0, 0.0],
                angular_velocity: [0.0, 0.0, 0.0],
                battery_level: 100.0,
                status: AgentStatus::Idle,
            },
            model: config.model.clone(),
            capabilities: config.capabilities.clone(),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn get_state(&self) -> AgentState {
        self.state.clone()
    }

    pub fn update(&mut self, physics_state: &PhysicsState, dt: f64) -> Result<AgentState> {
        // Update position based on velocity
        for i in 0..3 {
            self.state.position[i] += self.state.velocity[i] * dt;
        }

        // Update battery
        self.state.battery_level = (self.state.battery_level - dt * 0.01).max(0.0);

        Ok(self.state.clone())
    }
}

/// Simulation step result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationStep {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub time_step: f64,
    pub physics_state: PhysicsState,
    pub agent_states: Vec<AgentState>,
}

/// Simulation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationStats {
    pub engine: String,
    pub world_file: String,
    pub total_agents: i64,
    pub running: bool,
    pub time_step: f64,
    pub real_time_factor: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulation_manager() {
        let config = SimulationConfig::default();
        let manager = SimulationManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Add agent
        let agent_config = AgentConfig {
            id: "agent-1".to_string(),
            name: "Test Agent".to_string(),
            initial_position: [0.0, 0.0, 0.0],
            initial_orientation: [1.0, 0.0, 0.0, 0.0],
            model: "turtlebot3".to_string(),
            capabilities: vec!["navigation".to_string()],
        };

        let agent_id = manager.add_agent(agent_config).await.unwrap();
        assert!(!agent_id.is_empty());

        // Run simulation
        manager.start().await.unwrap();
        let steps = manager.run_for(1.0).await.unwrap();
        assert!(!steps.is_empty());

        // Get stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_agents, 1);
        assert!(stats.running);
    }
}
