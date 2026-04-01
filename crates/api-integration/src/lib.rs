//! External API integration (REST, gRPC, GraphQL, WebSocket) for multi-agent systems.
//!
//! This crate provides adapters for connecting the multi-agent system with
//! external services through various API protocols.
//!
//! # Features
//! - **REST API client** with retry logic, authentication, and flexible configuration
//! - **gRPC client and server** support (optional feature `grpc`) using tonic/prost
//! - **GraphQL client and server** support (optional feature `graphql`) using async-graphql
//! - **WebSocket** client and server (optional feature `websocket`) for real‑time communication
//! - **Integration with mesh transport** for bridging external APIs with agent networks
//!
//! # Examples
//!
//! ## REST Client
//! ```
//! use api_integration::RestClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RestClient::with_base_url("https://api.example.com")?;
//!
//!     // Make a GET request
//!     let response: serde_json::Value = client.get("/users/1").await?;
//!     println!("User: {:?}", response);
//!
//!     // Make a POST request
//!     let new_user = serde_json::json!({"name": "Alice", "email": "alice@example.com"});
//!     let created: serde_json::Value = client.post("/users", &new_user).await?;
//!     println!("Created user: {:?}", created);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## GraphQL Server
//! ```ignore
//! use api_integration::graphql::{GraphQLServer, AgentRegistry, TaskStore};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let registry = Arc::new(MyAgentRegistry::new());
//!     let task_store = Arc::new(MyTaskStore::new());
//!
//!     let server = GraphQLServer::new(
//!         "127.0.0.1:8080".parse()?,
//!         registry,
//!         task_store,
//!     );
//!
//!     server.serve().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## gRPC Client
//! ```ignore
//! #[cfg(feature = "grpc")]
//! use api_integration::grpc::GrpcClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = GrpcClient::connect("http://localhost:50051").await?;
//!     // Use client...
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod rest;

// Re-export commonly used types
pub use error::{ApiIntegrationError, Result};
pub use rest::{RestApiAdapter, RestClient, RestClientConfig, SimpleRestAdapter};

/// Current version of the API integration crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the API integration system.
pub fn init() {
    // Any initialization logic would go here
    tracing::info!("API Integration v{} initialized", VERSION);
}

// gRPC module (conditionally compiled)
#[cfg(feature = "grpc")]
pub mod grpc;

// GraphQL module (conditionally compiled)
#[cfg(feature = "graphql")]
pub mod graphql;

// WebSocket module (conditionally compiled)
#[cfg(feature = "websocket")]
pub mod websocket;

/// Re-exports for conditional features
#[cfg(feature = "grpc")]
pub use grpc::{GrpcClient, GrpcHandler, GrpcMeshBridge, GrpcServer};

#[cfg(feature = "graphql")]
pub use graphql::{
    Agent, AgentRegistry, GraphQLClient, GraphQLMeshIntegration, GraphQLServer, HealthStatus,
    QueryRoot, ResourceMetrics, Task, TaskStore,
};

#[cfg(feature = "websocket")]
pub use websocket::{
    ClientMessage, ServerEvent, ServerMessage, WebSocketClient, WebSocketMeshBridge,
    WebSocketServer,
};

/// Unified API client that can work with multiple protocols.
pub struct UnifiedApiClient {
    /// REST client (always available)
    pub rest: RestClient,
    /// gRPC client (if feature enabled)
    #[cfg(feature = "grpc")]
    pub grpc: Option<GrpcClient>,
    /// GraphQL client (if feature enabled)
    #[cfg(feature = "graphql")]
    pub graphql: Option<GraphQLClient>,
    /// WebSocket client (if feature enabled)
    #[cfg(feature = "websocket")]
    pub websocket: Option<WebSocketClient>,
}

impl UnifiedApiClient {
    /// Create a new unified client with REST only.
    pub fn new() -> Result<Self> {
        Ok(Self {
            rest: RestClient::new()?,
            #[cfg(feature = "grpc")]
            grpc: None,
            #[cfg(feature = "graphql")]
            graphql: None,
            #[cfg(feature = "websocket")]
            websocket: None,
        })
    }

    /// Connect a gRPC client.
    #[cfg(feature = "grpc")]
    pub async fn connect_grpc(&mut self, endpoint: &str) -> Result<()> {
        let client = GrpcClient::connect(endpoint).await?;
        self.grpc = Some(client);
        Ok(())
    }

    /// Connect a GraphQL client.
    #[cfg(feature = "graphql")]
    pub fn connect_graphql(&mut self, endpoint: &str) {
        self.graphql = Some(GraphQLClient::new(endpoint));
    }

    /// Connect a WebSocket client.
    #[cfg(feature = "websocket")]
    pub async fn connect_websocket(&mut self, url: &str) -> Result<()> {
        let client = WebSocketClient::new(url);
        let (tx, _) = client.connect().await?;
        // Store connection
        self.websocket = Some(client);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_rest_client_creation() {
        let client = RestClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_rest_client_config_default() {
        let config = RestClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 1000);
        assert!(config.auth_token.is_none());
        assert!(config.headers.is_empty());
    }

    #[test]
    fn test_unified_client_creation() {
        let client = UnifiedApiClient::new();
        assert!(client.is_ok());
    }
}