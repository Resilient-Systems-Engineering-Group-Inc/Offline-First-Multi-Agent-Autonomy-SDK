//! Core agent that integrates transport and state synchronization.

pub mod agent;
pub mod integration;

pub use agent::Agent;
pub use integration::IntegrationAdapter;