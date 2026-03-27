//! RL agent that interacts with the environment.

use crate::environment::{PlanningEnvironment, State, Action, Reward};
use crate::policy::Policy;
use distributed_planner::{Task, AgentId};
use ndarray::Array1;

/// Reinforcement learning planner.
pub struct RlPlanner<P: Policy> {
    policy: P,
    environment: PlanningEnvironment,
}

impl<P: Policy> RlPlanner<P> {
    /// Create a new RL planner with a given policy and environment.
    pub fn new(policy: P, environment: PlanningEnvironment) -> Self {
        Self { policy, environment }
    }

    /// Plan assignments for the current pending tasks.
    /// Returns a list of assignments (task → agent).
    pub fn plan(&mut self) -> Vec<(Task, AgentId)> {
        let mut assignments = Vec::new();
        let mut state = self.environment.state().clone();

        while !state.pending_tasks.is_empty() {
            let action = self.policy.select_action(&state);
            // Find the task
            if let Some(task) = state.pending_tasks.iter()
                .find(|t| t.id == action.task_id)
                .cloned()
            {
                assignments.push((task, action.agent_id));
                // Simulate step (but we don't need reward for planning)
                state.pending_tasks.retain(|t| t.id != action.task_id);
            } else {
                break;
            }
        }
        assignments
    }

    /// Train the planner for one episode.
    pub fn train_episode(&mut self) -> Vec<Reward> {
        let mut rewards = Vec::new();
        let mut done = false;
        while !done {
            let state = self.environment.state().clone();
            let action = self.policy.select_action(&state);
            let (next_state, reward, episode_done) = self.environment.step(action.clone());
            rewards.push(reward);
            self.policy.update(&next_state, &action, reward);
            done = episode_done;
        }
        rewards
    }

    /// Get the underlying environment.
    pub fn environment(&self) -> &PlanningEnvironment {
        &self.environment
    }

    /// Get a mutable reference to the environment.
    pub fn environment_mut(&mut self) -> &mut PlanningEnvironment {
        &mut self.environment
    }

    /// Get the policy.
    pub fn policy(&self) -> &P {
        &self.policy
    }

    /// Get a mutable reference to the policy.
    pub fn policy_mut(&mut self) -> &mut P {
        &mut self.policy
    }
}