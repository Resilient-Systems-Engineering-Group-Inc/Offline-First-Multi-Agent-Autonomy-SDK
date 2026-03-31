//! Digital twin simulation of the physical world.
//!
//! This crate provides a virtual representation of the physical environment,
//! integrating IoT sensors, actuators, and agent interactions.

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod model;
pub mod physics;
pub mod visualization;

pub use error::Error;
pub use model::DigitalTwin;