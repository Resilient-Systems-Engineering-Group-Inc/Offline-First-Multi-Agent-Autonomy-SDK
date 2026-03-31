//! Error types for dependency graph operations.

use thiserror::Error;

/// Errors that can occur while working with dependency graphs.
#[derive(Error, Debug)]
pub enum DependencyError {
    /// A cycle was detected in the graph.
    #[error("cycle detected in dependency graph")]
    CycleDetected,

    /// Node not found.
    #[error("node {0} not found")]
    NodeNotFound(String),

    /// Edge not found.
    #[error("edge from {0} to {1} not found")]
    EdgeNotFound(String, String),

    /// Invalid node data.
    #[error("invalid node data: {0}")]
    InvalidNodeData(String),

    /// Invalid edge data.
    #[error("invalid edge data: {0}")]
    InvalidEdgeData(String),

    /// Graph is not a DAG (directed acyclic graph).
    #[error("graph is not a DAG")]
    NotADag,

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error.
    #[error("other error: {0}")]
    Other(String),
}

impl DependencyError {
    /// Create a new cycle detected error.
    pub fn cycle() -> Self {
        Self::CycleDetected
    }

    /// Create a new node not found error.
    pub fn node_not_found(id: impl Into<String>) -> Self {
        Self::NodeNotFound(id.into())
    }

    /// Create a new edge not found error.
    pub fn edge_not_found(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::EdgeNotFound(from.into(), to.into())
    }
}