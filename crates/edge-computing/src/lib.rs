//! Edge computing optimizations for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides hardware‑aware adaptations, resource‑constrained
//! scheduling, and platform‑specific backends for edge devices.

#![deny(missing_docs, unsafe_code)]

pub mod hardware;
pub mod optimizer;
pub mod scheduler;
pub mod error;

pub use error::Error;
pub use optimizer::EdgeOptimizer;