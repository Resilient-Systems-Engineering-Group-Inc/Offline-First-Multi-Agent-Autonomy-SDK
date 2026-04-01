//! Error types for geospatial operations.

use thiserror::Error;

/// Errors that can occur in geospatial operations.
#[derive(Error, Debug)]
pub enum GeospatialError {
    /// Invalid coordinates.
    #[error("Invalid coordinates: {0}")]
    InvalidCoordinates(String),

    /// Invalid bounding box.
    #[error("Invalid bounding box: {0}")]
    InvalidBoundingBox(String),

    /// Distance calculation error.
    #[error("Distance calculation error: {0}")]
    DistanceError(String),

    /// Projection error.
    #[error("Projection error: {0}")]
    ProjectionError(String),

    /// Spatial index error.
    #[error("Spatial index error: {0}")]
    SpatialIndexError(String),

    /// Path planning error.
    #[error("Path planning error: {0}")]
    PathPlanningError(String),

    /// No path found.
    #[error("No path found from {0:?} to {1:?}")]
    NoPathFound((f64, f64), (f64, f64)),

    /// Agent not found in spatial registry.
    #[error("Agent {0} not found in spatial registry")]
    AgentNotFound(u64),

    /// Location update error.
    #[error("Location update error: {0}")]
    LocationUpdateError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for geospatial operations.
pub type Result<T> = std::result::Result<T, GeospatialError>;