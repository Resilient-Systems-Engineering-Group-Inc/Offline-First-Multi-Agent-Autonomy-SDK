//! RL environment for task planning.

use distributed_planner::{Task, Assignment, AgentId};
use ndarray::Array1;
use serde::{Deserialize, Serialize};

/// State of the planning environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Vector representation of the state (normalized features).
    pub features: Array1<f32>,
    /// List of pending tasks.
    pub pending_tasks: Vec<Task>,
    /// Available agents and their capabilities.
    pub available_agents: Vec<AgentId>,
    /// Current resource usage per agent (CPU, memory, etc.)
    pub resource_usage: Vec<f32>,
}

/// An action corresponds to assigning a task to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub task_id: u64,
    pub agent_id: AgentId,
}

/// Reward signal after taking an action.
pub type Reward = f32;

/// Environment that simulates the planning process.
pub struct PlanningEnvironment {
    /// Current state.
    state: State,
    /// History of states, actions, rewards.
    history: Vec<(State, Action, Reward)>,
    /// Maximum steps per episode.
    max_steps: usize,
    step_count: usize,
}

impl PlanningEnvironment {
    /// Create a new environment with an initial state.
    pub fn new(initial_state: State, max_steps: usize) -> Self {
        Self {
            state: initial_state,
            history: Vec::new(),
            max_steps,
            step_count: 0,
        }
    }

    /// Reset the environment to a given state.
    pub fn reset(&mut self, state: State) {
        self.state = state;
        self.history.clear();
        self.step_count = 0;
    }

    /// Step the environment by taking an action.
    /// Returns (next_state, reward, done).
    pub fn step(&mut self, action: Action) -> (State, Reward, bool) {
        // Simulate the effect of assigning the task.
        // For now, we just compute a dummy reward.
        let reward = self.compute_reward(&action);
        self.history.push((self.state.clone(), action, reward));

        // Update state (simplified: remove the assigned task)
        self.state.pending_tasks.retain(|t| t.id != action.task_id);
        self.step_count += 1;

        let done = self.step_count >= self.max_steps || self.state.pending_tasks.is_empty();
        (self.state.clone(), reward, done)
    }

    /// Compute reward based on action.
    fn compute_reward(&self, action: &Action) -> Reward {
        // Reward is higher if the agent has low resource usage.
        let agent_idx = self.state.available_agents.iter()
            .position(|&id| id == action.agent_id)
            .unwrap_or(0);
        let usage = self.state.resource_usage.get(agent_idx).cloned().unwrap_or(0.0);
        // Lower usage -> higher reward
        1.0 - usage
    }

    /// Get current state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get history.
    pub fn history(&self) -> &[(State, Action, Reward)] {
        &self.history
    }
}