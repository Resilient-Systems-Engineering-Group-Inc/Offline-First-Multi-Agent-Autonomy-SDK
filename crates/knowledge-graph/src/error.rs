//! Error types for knowledge graph operations.

use thiserror::Error;

/// Errors that can occur in knowledge graph operations.
#[derive(Error, Debug)]
pub enum KnowledgeGraphError {
    /// Entity not found.
    #[error("Entity {0} not found")]
    EntityNotFound(String),

    /// Relationship not found.
    #[error("Relationship {0} not found")]
    RelationshipNotFound(String),

    /// Entity already exists.
    #[error("Entity {0} already exists")]
    EntityAlreadyExists(String),

    /// Relationship already exists.
    #[error("Relationship {0} already exists")]
    RelationshipAlreadyExists(String),

    /// Invalid entity ID.
    #[error("Invalid entity ID: {0}")]
    InvalidEntityId(String),

    /// Invalid relationship type.
    #[error("Invalid relationship type: {0}")]
    InvalidRelationshipType(String),

    /// Invalid property.
    #[error("Invalid property: {0}")]
    InvalidProperty(String),

    /// Query parsing error.
    #[error("Query parsing error: {0}")]
    QueryParseError(String),

    /// Query execution error.
    #[error("Query execution error: {0}")]
    QueryExecutionError(String),

    /// Graph traversal error.
    #[error("Graph traversal error: {0}")]
    TraversalError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for knowledge graph operations.
pub type Result<T> = std::result::Result<T, KnowledgeGraphError>;