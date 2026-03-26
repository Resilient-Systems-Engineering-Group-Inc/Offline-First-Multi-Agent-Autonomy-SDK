//! Local planner for autonomous decision‑making.

use common::types::AgentId;
use common::error::Result;
use async_trait::async_trait;
use state_sync::StateSync;
use mesh_transport::Transport;

/// A local planner decides what actions an agent should take based on its
/// current state and the state of its peers.
#[async_trait]
pub trait LocalPlanner: Send + Sync {
    /// Plan the next action(s) for the agent.
    async fn plan(&mut self) -> Result<Vec<Action>>;

    /// Execute a planned action.
    async fn execute(&mut self, action: Action) -> Result<()>;

    /// Update the planner with new state from the environment.
    async fn update_state(&mut self, state: serde_json::Value) -> Result<()>;
}

/// An action that an agent can perform.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Action {
    /// Move to a target position (x, y, z).
    MoveTo(f64, f64, f64),
    /// Pick up an object with given ID.
    PickUp(String),
    /// Drop the currently held object.
    Drop,
    /// Communicate a message to another agent.
    Communicate(AgentId, serde_json::Value),
    /// Wait for a given duration (seconds).
    Wait(f64),
    /// No‑op.
    NoOp,
}

/// A simple planner that does nothing (placeholder).
pub struct NoOpPlanner;

#[async_trait]
impl LocalPlanner for NoOpPlanner {
    async fn plan(&mut self) -> Result<Vec<Action>> {
        Ok(vec![Action::NoOp])
    }

    async fn execute(&mut self, _action: Action) -> Result<()> {
        Ok(())
    }

    async fn update_state(&mut self, _state: serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// A planner that uses the agent's CRDT map to decide.
pub struct MapBasedPlanner<T: Transport, S: StateSync> {
    transport: T,
    state_sync: S,
    goal: Option<serde_json::Value>,
}

impl<T: Transport, S: StateSync> MapBasedPlanner<T, S> {
    /// Create a new map‑based planner.
    pub fn new(transport: T, state_sync: S) -> Self {
        Self {
            transport,
            state_sync,
            goal: None,
        }
    }

    /// Set a goal for the planner.
    pub fn set_goal(&mut self, goal: serde_json::Value) {
        self.goal = Some(goal);
    }
}

#[async_trait]
impl<T: Transport, S: StateSync> LocalPlanner for MapBasedPlanner<T, S> {
    async fn plan(&mut self) -> Result<Vec<Action>> {
        // Simple planning: if there's a goal, try to achieve it.
        // This is a placeholder implementation.
        if let Some(goal) = &self.goal {
            tracing::info!("Planning to achieve goal: {:?}", goal);
            // For demonstration, just produce a Communicate action.
            let action = Action::Communicate(AgentId(0), goal.clone());
            Ok(vec![action])
        } else {
            Ok(vec![Action::NoOp])
        }
    }

    async fn execute(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Communicate(peer, message) => {
                let payload = serde_json::to_vec(&message)?;
                self.transport.send_to(peer, payload).await?;
                Ok(())
            }
            _ => {
                tracing::warn!("Action {:?} not implemented", action);
                Ok(())
            }
        }
    }

    async fn update_state(&mut self, _state: serde_json::Value) -> Result<()> {
        // In a real implementation, we would update internal state.
        Ok(())
    }
}