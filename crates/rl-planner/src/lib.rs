//! Reinforcement learning for adaptive task planning.
//!
//! This crate provides RL‑based algorithms that can learn to make better
//! task‑assignment decisions over time, based on observed system performance.

pub mod environment;
pub mod agent;
pub mod policy;
pub mod trainer;

pub use environment::{PlanningEnvironment, State, Action, Reward};
pub use agent::RlPlanner;
pub use policy::{Policy, RandomPolicy, EpsilonGreedyPolicy};
pub use trainer::Trainer;

/// Pre‑import of commonly used types.
pub mod prelude {
    pub use crate::{RlPlanner, PlanningEnvironment, Policy, Trainer};
}