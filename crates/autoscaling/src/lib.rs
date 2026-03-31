//! Automatic scaling for multi‑agent swarms.
//!
//! This crate provides mechanisms to dynamically scale the number of agents
//! based on workload, resource utilization, and performance metrics.

pub mod error;
pub mod policy;
pub mod controller;
pub mod metrics;
pub mod scaler;

pub use error::AutoscalingError;
pub use policy::{ScalingPolicy, ThresholdPolicy, PredictivePolicy};
pub use controller::AutoscalingController;
pub use scaler::{Scaler, AgentScaler};

/// Re‑export of common types.
pub use common::types::AgentId;