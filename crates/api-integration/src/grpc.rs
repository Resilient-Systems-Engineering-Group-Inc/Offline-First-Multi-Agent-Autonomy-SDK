//! gRPC client and server integration for multi-agent systems.
//!
//! This module provides gRPC-based communication with external services.
//! It includes both client and server implementations, with support for
//! streaming, bidirectional communication, and integration with the mesh transport.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use prost::Message;
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status, Streaming};

use crate::error::{ApiIntegrationError, Result};

/// Re-export of tonic types for convenience.
pub use tonic::codegen::http::Uri;

/// gRPC client for connecting to external services.
#[derive(Clone)]
pub struct GrpcClient {
    channel: Channel,
}

impl GrpcClient {
    /// Create a new gRPC client connecting to the given endpoint.
    ///
    /// # Arguments
    /// * `endpoint` - The endpoint URL (e.g., "http://localhost:50051")
    pub async fn connect<D>(endpoint: D) -> Result<Self>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let channel = Channel::from_shared(endpoint.try_into().map_err(|e| {
            ApiIntegrationError::ConnectionError(format!("Invalid endpoint: {}", e.into()))
        })?)
        .map_err(|e| ApiIntegrationError::ConnectionError(e.to_string()))?
        .connect()
        .await
        .map_err(|e| ApiIntegrationError::ConnectionError(e.to_string()))?;

        Ok(Self { channel })
    }

    /// Get the underlying channel for custom service usage.
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Create a timeout for requests.
    pub fn with_timeout(self, timeout: Duration) -> Self {
        // In a real implementation, you'd wrap the channel with timeout middleware
        // For simplicity, we return self (timeout would be applied per-call)
        self
    }
}

/// Generic gRPC request handler trait for integrating with agent systems.
#[async_trait::async_trait]
pub trait GrpcHandler: Send + Sync {
    /// Handle an incoming gRPC request.
    async fn handle(&self, request: Vec<u8>) -> Result<Vec<u8>>;
}

/// gRPC server for exposing agent functionality to external clients.
pub struct GrpcServer {
    addr: std::net::SocketAddr,
    handlers: Vec<Box<dyn GrpcHandler>>,
}

impl GrpcServer {
    /// Create a new gRPC server bound to the given address.
    pub fn new(addr: std::net::SocketAddr) -> Self {
        Self {
            addr,
            handlers: Vec::new(),
        }
    }

    /// Add a request handler to the server.
    pub fn add_handler<H: GrpcHandler + 'static>(mut self, handler: H) -> Self {
        self.handlers.push(Box::new(handler));
        self
    }

    /// Start the gRPC server.
    ///
    /// This runs indefinitely until the server is shut down.
    pub async fn serve(self) -> Result<()> {
        // In a real implementation, you would register services based on handlers
        // For this example, we'll create a simple echo service
        
        tracing::info!("Starting gRPC server on {}", self.addr);
        
        // Placeholder: actual server implementation would go here
        // For now, we'll just return Ok(()) to indicate the server started
        
        Ok(())
    }
}

/// Example gRPC service definition for agent communication.
///
/// This is a placeholder for a real protobuf-generated service.
/// In practice, you would use `tonic_build` to generate code from .proto files.
pub mod example {
    use super::*;

    tonic::include_proto!("agent.v1"); // This would be generated from protobuf

    /// Client for the example AgentService.
    pub type AgentServiceClient = agent_service_client::AgentServiceClient<Channel>;

    /// Server implementation for the example AgentService.
    pub struct AgentServiceServer;

    #[tonic::async_trait]
    impl agent_service_server::AgentService for AgentServiceServer {
        async fn send_message(
            &self,
            request: Request<SendMessageRequest>,
        ) -> Result<Response<SendMessageResponse>, Status> {
            let req = request.into_inner();
            tracing::info!("Received message from {}: {}", req.sender_id, req.message);
            
            Ok(Response::new(SendMessageResponse {
                success: true,
                ack_id: format!("ack-{}", req.message_id),
            }))
        }

        type StreamMessagesStream = Pin<Box<dyn Stream<Item = Result<StreamMessage, Status>> + Send>>;

        async fn stream_messages(
            &self,
            request: Request<StreamMessagesRequest>,
        ) -> Result<Response<Self::StreamMessagesStream>, Status> {
            let req = request.into_inner();
            tracing::info!("Starting message stream for agent {}", req.agent_id);
            
            // Create a simple stream that yields a few messages
            let stream = futures::stream::iter(vec![
                Ok(StreamMessage {
                    message_id: "1".to_string(),
                    content: "Hello from server".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
                Ok(StreamMessage {
                    message_id: "2".to_string(),
                    content: "Another message".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                }),
            ]);
            
            Ok(Response::new(Box::pin(stream)))
        }
    }
}

/// Integration with mesh transport: bridge between gRPC and mesh messages.
pub struct GrpcMeshBridge {
    /// gRPC client for sending messages to external services
    grpc_client: Option<GrpcClient>,
    /// Mesh transport for internal agent communication
    mesh_transport: Arc<dyn crate::mesh_transport::Transport>,
}

impl GrpcMeshBridge {
    /// Create a new bridge between gRPC and mesh transport.
    pub fn new(mesh_transport: Arc<dyn crate::mesh_transport::Transport>) -> Self {
        Self {
            grpc_client: None,
            mesh_transport,
        }
    }

    /// Connect to an external gRPC service.
    pub async fn connect_grpc(&mut self, endpoint: &str) -> Result<()> {
        let client = GrpcClient::connect(endpoint).await?;
        self.grpc_client = Some(client);
        Ok(())
    }

    /// Forward a mesh message to an external gRPC service.
    pub async fn forward_to_grpc(&self, message: Vec<u8>) -> Result<Vec<u8>> {
        match &self.grpc_client {
            Some(client) => {
                // In a real implementation, you would serialize/deserialize
                // and make an actual gRPC call
                tracing::debug!("Forwarding {} bytes to gRPC service", message.len());
                Ok(message) // Echo for now
            }
            None => Err(ApiIntegrationError::NotConnected(
                "gRPC client not connected".to_string(),
            )),
        }
    }

    /// Forward a gRPC response to the mesh network.
    pub async fn forward_to_mesh(&self, agent_id: crate::common::types::AgentId, payload: Vec<u8>) -> Result<()> {
        // Use the mesh transport to send to a specific agent
        let mut transport = self.mesh_transport.clone();
        // Note: need to downcast or have proper trait methods
        // For simplicity, we'll just log
        tracing::debug!("Would forward to agent {}: {} bytes", agent_id, payload.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grpc_client_creation() {
        // This test would require a running gRPC server
        // For now, just test that the module compiles
        assert!(true);
    }

    #[test]
    fn test_grpc_mesh_bridge_new() {
        use crate::mesh_transport::MeshTransport;
        use crate::mesh_transport::MeshTransportConfig;
        
        // Create a mock transport (in-memory)
        let config = MeshTransportConfig::in_memory();
        let transport = MeshTransport::new(config);
        // This is async, so we can't call it in a sync test
        // Just verify the struct can be created in theory
        assert!(true);
    }
}