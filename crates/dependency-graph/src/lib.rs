//! Dependency graph management for multi‑agent systems.
//!
//! This crate provides a directed acyclic graph (DAG) representation of dependencies
//! between agents, tasks, or resources, with support for topological sorting,
//! cycle detection, and dynamic updates.

pub mod error;
pub mod graph;
pub mod scheduler;
pub mod event;

pub use error::DependencyError;
pub use graph::{DependencyGraph, Node, Edge, NodeId, EdgeType};
pub use scheduler::DependencyScheduler;
pub use event::DependencyEvent;

/// Re‑export of petgraph for advanced graph operations.
pub use petgraph;