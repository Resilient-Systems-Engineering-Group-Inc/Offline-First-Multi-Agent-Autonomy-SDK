//! Policies for action selection.

use crate::environment::{State, Action};
use rand::Rng;

/// Trait for a policy that selects actions given a state.
pub trait Policy: Send + Sync {
    /// Select an action.
    fn select_action(&self, state: &State) -> Action;

    /// Update the policy based on experience (if learning).
    fn update(&mut self, _state: &State, _action: &Action, _reward: f32) {}
}

/// Random policy (baseline).
pub struct RandomPolicy;

impl Policy for RandomPolicy {
    fn select_action(&self, state: &State) -> Action {
        let mut rng = rand::thread_rng();
        // Pick a random pending task
        let task = if state.pending_tasks.is_empty() {
            // No tasks, return a dummy action
            return Action { task_id: 0, agent_id: distributed_planner::AgentId(0) };
        } else {
            &state.pending_tasks[rng.gen_range(0..state.pending_tasks.len())]
        };
        // Pick a random agent
        let agent = if state.available_agents.is_empty() {
            distributed_planner::AgentId(0)
        } else {
            state.available_agents[rng.gen_range(0..state.available_agents.len())]
        };
        Action { task_id: task.id, agent_id: agent }
    }
}

/// Epsilon‑greedy policy.
pub struct EpsilonGreedyPolicy<P: Policy> {
    inner: P,
    epsilon: f32,
}

impl<P: Policy> EpsilonGreedyPolicy<P> {
    pub fn new(inner: P, epsilon: f32) -> Self {
        Self { inner, epsilon }
    }

    pub fn set_epsilon(&mut self, epsilon: f32) {
        self.epsilon = epsilon;
    }
}

impl<P: Policy> Policy for EpsilonGreedyPolicy<P> {
    fn select_action(&self, state: &State) -> Action {
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < self.epsilon {
            // Random action
            RandomPolicy.select_action(state)
        } else {
            // Greedy action from inner policy
            self.inner.select_action(state)
        }
    }

    fn update(&mut self, state: &State, action: &Action, reward: f32) {
        self.inner.update(state, action, reward);
    }
}