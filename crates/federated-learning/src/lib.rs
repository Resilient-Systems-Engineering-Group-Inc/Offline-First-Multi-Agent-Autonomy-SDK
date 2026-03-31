//! Federated learning for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate enables privacy‑preserving distributed machine learning across
//! agents without centralizing raw data.

#![deny(missing_docs, unsafe_code)]

pub mod aggregation;
pub mod client;
pub mod error;
pub mod model;
pub mod server;
pub mod privacy;

pub use error::Error;
pub use client::FederatedClient;
pub use server::FederatedServer;