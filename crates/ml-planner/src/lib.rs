//! ML-based task planning with reinforcement learning.
//!
//! Provides:
//! - Q-Learning for task assignment
//! - Deep Q-Networks (DQN)
//! - Multi-Agent RL
//! - Adaptive planning

pub mod algorithms;
pub mod model;
pub mod trainer;

use anyhow::Result;
use std::path::Path;
use tokio::sync::RwLock;
use tracing::info;

pub use algorithms::*;
pub use model::*;
pub use trainer::*;

/// Q-Learning based planner.
pub struct QLearningPlanner {
    learning_rate: f64,
    discount_factor: f64,
    exploration_rate: f64,
    q_table: RwLock<std::collections::HashMap<String, Vec<f64>>>,
}

impl QLearningPlanner {
    /// Create new Q-Learning planner.
    pub fn new(
        learning_rate: f64,
        discount_factor: f64,
        exploration_rate: f64,
    ) -> Self {
        Self {
            learning_rate,
            discount_factor,
            exploration_rate,
            q_table: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get Q-value for state-action pair.
    pub async fn get_q_value(&self, state: &str, action: usize) -> f64 {
        let q_table = self.q_table.read().await;
        q_table
            .get(state)
            .map(|values| values.get(action).copied().unwrap_or(0.0))
            .unwrap_or(0.0)
    }

    /// Update Q-value for state-action pair.
    pub async fn update_q_value(
        &mut self,
        state: &str,
        action: usize,
        reward: f64,
        next_state: &str,
        actions_count: usize,
    ) {
        let mut q_table = self.q_table.write().await;

        // Initialize Q-values if not exists
        let entry = q_table.entry(state.to_string()).or_insert_with(|| {
            vec![0.0; actions_count]
        });

        // Get max Q-value for next state
        let next_max = q_table
            .get(next_state)
            .map(|values| values.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
            .unwrap_or(0.0);

        // Q-learning update formula
        let current_q = entry.get(action).copied().unwrap_or(0.0);
        let new_q = current_q + self.learning_rate * (reward
            + self.discount_factor * next_max
            - current_q);

        if let Some(val) = entry.get_mut(action) {
            *val = new_q;
        }
    }

    /// Select action using epsilon-greedy policy.
    pub async fn select_action(&self, state: &str, actions_count: usize) -> usize {
        let mut rng = rand::thread_rng();

        // Exploration
        if rng.gen::<f64>() < self.exploration_rate {
            return rng.gen::<usize>() % actions_count;
        }

        // Exploitation
        let q_table = self.q_table.read().await;
        match q_table.get(state) {
            Some(values) => values
                .iter()
                .position(|&q| q == values.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
                .unwrap_or(0),
            None => rng.gen::<usize>() % actions_count,
        }
    }

    /// Save Q-table to file.
    pub async fn save(&self, path: &Path) -> Result<()> {
        let q_table = self.q_table.read().await;
        let json = serde_json::to_string_pretty(&*q_table)?;
        tokio::fs::write(path, json).await?;
        info!("Q-table saved to {:?}", path);
        Ok(())
    }

    /// Load Q-table from file.
    pub async fn load(&mut self, path: &Path) -> Result<()> {
        let json = tokio::fs::read_to_string(path).await?;
        let q_table: std::collections::HashMap<String, Vec<f64>> = serde_json::from_str(&json)?;
        
        let mut table = self.q_table.write().await;
        *table = q_table;
        
        info!("Q-table loaded from {:?}", path);
        Ok(())
    }
}

/// Deep Q-Network planner.
pub struct DQNPlanner {
    model: RwLock<Option<torch::nn::Module>>,
    device: torch::Device,
    batch_size: usize,
}

impl DQNPlanner {
    /// Create new DQN planner.
    pub fn new(use_cuda: bool) -> Self {
        let device = if use_cuda && torch::CudaDevice::count() > 0 {
            torch::Device::Cuda(0)
        } else {
            torch::Device::Cpu
        };

        Self {
            model: RwLock::new(None),
            device,
            batch_size: 64,
        }
    }

    /// Initialize the DQN model.
    pub async fn initialize(&self, input_dim: usize, output_dim: usize) -> Result<()> {
        // Simple DQN architecture
        let mut model = torch::nn::Sequential::new();
        
        // Input layer
        model.push(torch::nn::Linear::new(
            input_dim as i64,
            128,
            Default::default(),
        ));
        model.push(torch::nn::functional::torch::relu);
        
        // Hidden layers
        model.push(torch::nn::Linear::new(128, 256, Default::default()));
        model.push(torch::nn::functional::torch::relu);
        
        model.push(torch::nn::Linear::new(256, 128, Default::default()));
        model.push(torch::nn::functional::torch::relu);
        
        // Output layer
        model.push(torch::nn::Linear::new(128, output_dim as i64, Default::default()));

        let mut model = torch::nn::ModuleHolder::new(model);
        model.to(&self.device);

        let mut model_lock = self.model.write().await;
        *model_lock = Some(model);

        info!("DQN model initialized on {:?}", self.device);
        Ok(())
    }

    /// Predict Q-values for state.
    pub async fn predict(&self, state: &[f64]) -> Result<Vec<f64>> {
        let model = self.model.read().await;
        match model.as_ref() {
            Some(m) => {
                // Convert state to tensor
                let state_tensor = torch::Tensor::from_slice(state)
                    .to(&self.device)
                    .unsqueeze(0)?;

                // Forward pass
                let output = m.forward(&state_tensor)?;
                let q_values = output
                    .contiguous()?
                    .to_kind(torch::Kind::Float)
                    .last_dim()?
                    .to_vec1::<f64>()?;

                Ok(q_values)
            }
            None => Err(anyhow::anyhow!("Model not initialized")),
        }
    }

    /// Train the model.
    pub async fn train(
        &self,
        states: &[Vec<f64>],
        actions: &[usize],
        rewards: &[f64],
        next_states: &[Vec<f64>],
        episodes: usize,
    ) -> Result<()> {
        info!("Training DQN for {} episodes", episodes);
        
        // Training loop would go here
        // For now, placeholder
        
        Ok(())
    }
}

/// Multi-Agent RL planner.
pub struct MultiAgentRLPlanner {
    agents: RwLock<std::collections::HashMap<String, QLearningPlanner>>,
    shared_experience: RwLock<Vec<Experience>,
}

impl MultiAgentRLPlanner {
    /// Create new multi-agent RL planner.
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(std::collections::HashMap::new()),
            shared_experience: RwLock::new(Vec::new()),
        }
    }

    /// Register agent.
    pub async fn register_agent(&self, agent_id: &str) {
        let mut agents = self.agents.write().await;
        agents.insert(
            agent_id.to_string(),
            QLearningPlanner::new(0.1, 0.95, 0.2),
        );
    }

    /// Add experience to shared replay buffer.
    pub async fn add_experience(&self, experience: Experience) {
        let mut experiences = self.shared_experience.write().await;
        experiences.push(experience);
        
        // Keep buffer bounded
        if experiences.len() > 10000 {
            experiences.remove(0);
        }
    }

    /// Sample batch of experiences.
    pub async fn sample_batch(&self, batch_size: usize) -> Vec<Experience> {
        let experiences = self.shared_experience.read().await;
        let mut rng = rand::thread_rng();
        
        (0..batch_size.min(experiences.len()))
            .map(|_| {
                let idx = rng.gen::<usize>() % experiences.len();
                experiences[idx].clone()
            })
            .collect()
    }

    /// Update all agents with shared experience.
    pub async fn update_all_agents(&self, experiences: &[Experience]) {
        let mut agents = self.agents.write().await;
        
        for exp in experiences {
            for (agent_id, planner) in agents.iter_mut() {
                // Update Q-values based on experience
                planner.update_q_value(
                    &exp.state,
                    exp.action,
                    exp.reward,
                    &exp.next_state,
                    exp.actions_count,
                ).await;
            }
        }
    }
}

impl Default for MultiAgentRLPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Experience tuple for RL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Experience {
    pub state: String,
    pub action: usize,
    pub reward: f64,
    pub next_state: String,
    pub done: bool,
    pub actions_count: usize,
}

impl Experience {
    pub fn new(
        state: &str,
        action: usize,
        reward: f64,
        next_state: &str,
        done: bool,
        actions_count: usize,
    ) -> Self {
        Self {
            state: state.to_string(),
            action,
            reward,
            next_state: next_state.to_string(),
            done,
            actions_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_q_learning() {
        let mut planner = QLearningPlanner::new(0.1, 0.95, 0.2);

        // Update Q-value
        planner.update_q_value("state1", 0, 1.0, "state2", 3).await;

        // Get Q-value
        let q_value = planner.get_q_value("state1", 0).await;
        assert!(q_value > 0.0);

        // Select action
        let action = planner.select_action("state1", 3).await;
        assert!(action < 3);
    }

    #[tokio::test]
    async fn test_q_table_save_load() {
        let mut planner = QLearningPlanner::new(0.1, 0.95, 0.2);
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("q_table.json");

        // Update Q-value
        planner.update_q_value("state1", 0, 1.0, "state2", 3).await;

        // Save
        planner.save(&path).await.unwrap();

        // Create new planner and load
        let mut planner2 = QLearningPlanner::new(0.1, 0.95, 0.2);
        planner2.load(&path).await.unwrap();

        // Verify
        let q_value = planner2.get_q_value("state1", 0).await;
        assert!(q_value > 0.0);
    }
}
