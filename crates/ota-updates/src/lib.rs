//! Over‑the‑air updates for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides secure, delta‑based OTA updates for agents and
//! components across the mesh network.

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod manager;
pub mod package;
pub mod signature;
pub mod transport;

pub use error::Error;
pub use manager::UpdateManager;
pub use package::{Package, PackageId, Version};