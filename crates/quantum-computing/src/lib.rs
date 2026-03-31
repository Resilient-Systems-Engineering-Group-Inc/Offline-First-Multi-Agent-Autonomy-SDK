//! Quantum computing integration for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides quantum‑enhanced algorithms for optimization,
//! machine learning, and secure communication.

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod algorithm;
pub mod backend;
pub mod circuit;

pub use error::Error;
pub use backend::{QuantumBackend, SimulatorBackend};
pub use algorithm::{QuantumOptimizer, QuantumAnnealer};