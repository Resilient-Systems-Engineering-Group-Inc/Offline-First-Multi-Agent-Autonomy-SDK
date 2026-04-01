//! Error types for API integration.

use thiserror::Error;

/// Errors that can occur in API integration operations.
#[derive(Error, Debug)]
pub enum ApiIntegrationError {
    /// HTTP request error.
    #[error("HTTP request error: {0}")]
    HttpRequestError(#[from] reqwest::Error),

    /// URL parsing error.
    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// gRPC error.
    #[cfg(feature = "grpc")]
    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),

    /// GraphQL error.
    #[cfg(feature = "graphql")]
    #[error("GraphQL error: {0}")]
    GraphQLError(String),

    /// WebSocket error.
    #[cfg(feature = "websocket")]
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Authentication error.
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Rate limiting error.
    #[error("Rate limiting error: {0}")]
    RateLimitError(String),

    /// Timeout error.
    #[error("Timeout error: {0}")]
    TimeoutError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid response error.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Service unavailable error.
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for API integration operations.
pub type Result<T> = std::result::Result<T, ApiIntegrationError>;