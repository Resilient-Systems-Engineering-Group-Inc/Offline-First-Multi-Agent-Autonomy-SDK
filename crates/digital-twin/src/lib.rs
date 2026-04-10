//! Digital twin simulation of the physical world.
//!
//! This crate provides a virtual representation of the physical environment,
//! integrating IoT sensors, actuators, and agent interactions.
//!
//! ## Modules
//! - `model`: Core data structures for digital twin entities and relationships.
//! - `physics`: Physics simulation (collisions, motion, forces).
//! - `visualization`: 2D/3D rendering and scene management.
//! - `error`: Error types.

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod model;
pub mod physics;
pub mod visualization;

pub use error::Error;
pub use model::{DigitalTwin, DigitalTwinModel, Entity, EntityState, EntityType, OperationalStatus, Property, Relationship};
pub use physics::{CollisionShape, PhysicsConfig, PhysicsEngine, PhysicalProperties};
pub use visualization::{Camera, Projection, Renderer, Scene, Simple2DRenderer, VisualRepresentation, VisualizationManager, create_2d_visualization};