//! Power management for edge devices.
//!
//! This crate provides energy‑aware scheduling, battery monitoring,
//! and dynamic power‑state adjustments.

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod monitor;
pub mod policy;
pub mod scheduler;
pub mod resource_manager;

pub use error::Error;
pub use monitor::PowerMonitor;
pub use policy::PowerPolicy;
pub use scheduler::PowerAwareScheduler;
pub use resource_manager::EnergyAwareResourceManager;