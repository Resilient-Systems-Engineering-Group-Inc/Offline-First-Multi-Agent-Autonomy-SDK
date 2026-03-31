//! Streaming data channels for real‑time communication between agents.
//!
//! This crate provides publish‑subscribe streams, QoS guarantees, compression,
//! and integration with the mesh transport.

#![deny(missing_docs, unsafe_code)]

pub mod channel;
pub mod codec;
pub mod error;
pub mod manager;
pub mod metrics;
pub mod qos;
pub mod subscription;

pub use channel::{Publisher, Subscriber};
pub use error::Error;
pub use manager::StreamManager;
pub use qos::{QoS, QualityOfService};