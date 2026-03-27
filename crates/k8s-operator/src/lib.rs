//! Kubernetes operator for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides a Kubernetes operator that manages custom resources
//! representing autonomous agents and tasks, reconciling them with the
//! underlying mesh network and agent core.

#![deny(missing_docs, unsafe_code)]

pub mod crd;
pub mod controller;
pub mod error;
pub mod reconciler;

pub use crd::{Agent, Task};
pub use error::Error;