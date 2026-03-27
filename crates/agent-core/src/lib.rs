//! Core agent that integrates transport and state synchronization.

pub mod agent;
pub mod integration;
pub mod fault_tolerance;

pub use agent::Agent;
pub use integration::IntegrationAdapter;
pub use fault_tolerance::{FaultDetector, TaskReallocator, FaultToleranceManager};