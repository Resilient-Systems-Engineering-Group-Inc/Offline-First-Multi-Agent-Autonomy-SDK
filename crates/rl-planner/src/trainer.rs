//! Training utilities for RL planner.

use crate::agent::RlPlanner;
use crate::policy::Policy;
use crate::environment::{PlanningEnvironment, State};
use tracing::{info, warn};

/// Trainer that runs multiple episodes and logs progress.
pub struct Trainer<P: Policy> {
    planner: RlPlanner<P>,
    num_episodes: usize,
    log_interval: usize,
}

impl<P: Policy> Trainer<P> {
    pub fn new(planner: RlPlanner<P>, num_episodes: usize, log_interval: usize) -> Self {
        Self {
            planner,
            num_episodes,
            log_interval,
        }
    }

    /// Run training.
    pub fn train(&mut self) -> Vec<f32> {
        let mut episode_rewards = Vec::new();
        for episode in 0..self.num_episodes {
            let rewards = self.planner.train_episode();
            let total_reward: f32 = rewards.iter().sum();
            episode_rewards.push(total_reward);

            if episode % self.log_interval == 0 {
                info!("Episode {}: total reward = {:.3}", episode, total_reward);
            }

            // Reset environment for next episode (for simplicity, we keep same state)
            // In a real scenario we would generate a new initial state.
        }
        episode_rewards
    }

    /// Evaluate the planner on a given environment.
    pub fn evaluate(&mut self, eval_env: PlanningEnvironment) -> f32 {
        let original_env = std::mem::replace(self.planner.environment_mut(), eval_env);
        let rewards = self.planner.train_episode();
        let total: f32 = rewards.iter().sum();
        // Restore original environment
        *self.planner.environment_mut() = original_env;
        total
    }

    /// Get a mutable reference to the planner.
    pub fn planner_mut(&mut self) -> &mut RlPlanner<P> {
        &mut self.planner
    }
}

// Helper to create a dummy state for testing.
pub fn dummy_state() -> State {
    use distributed_planner::{Task, AgentId};
    use ndarray::Array1;

    State {
        features: Array1::zeros(10),
        pending_tasks: vec![
            Task { id: 1, description: "task1".to_string(), required_capabilities: vec![], priority: 1, deadline: None, dependencies: vec![] },
            Task { id: 2, description: "task2".to_string(), required_capabilities: vec![], priority: 2, deadline: None, dependencies: vec![] },
        ],
        available_agents: vec![AgentId(1), AgentId(2), AgentId(3)],
        resource_usage: vec![0.3, 0.5, 0.8],
    }
}