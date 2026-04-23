//! API Gateway for the Multi-Agent SDK.
//!
//! Provides:
//! - Unified API entry point
//! - Rate limiting
//! - Authentication & Authorization
//! - Request/Response transformation
//! - Service discovery
//! - Load balancing

pub mod routes;
pub mod rate_limit;
pub mod middleware;
pub mod routing;

use anyhow::Result;
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::info;

pub use routes::*;
pub use rate_limit::*;
pub use middleware::*;
pub use routing::*;

/// Gateway configuration.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub bind_address: SocketAddr,
    pub rate_limit_requests: u32,
    pub rate_limit_window_secs: u64,
    pub enable_cors: bool,
    pub enable_compression: bool,
    pub services: Vec<ServiceConfig>,
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub url: String,
    pub weight: u32,
    pub health_check_path: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8000".parse().unwrap(),
            rate_limit_requests: 1000,
            rate_limit_window_secs: 60,
            enable_cors: true,
            enable_compression: true,
            services: vec![],
        }
    }
}

/// API Gateway.
pub struct ApiGateway {
    config: GatewayConfig,
    router: Router,
}

impl ApiGateway {
    /// Create new API gateway.
    pub fn new(config: GatewayConfig) -> Self {
        let router = Self::create_router(&config);
        
        Self {
            config,
            router,
        }
    }

    /// Create router with all routes.
    fn create_router(config: &GatewayConfig) -> Router {
        Router::new()
            // Health & metrics
            .route("/health", get(health_check))
            .route("/metrics", get(get_metrics))
            
            // API routes
            .route("/api/v1/tasks", get(list_tasks).post(create_task))
            .route("/api/v1/tasks/:id", get(get_task).put(update_task).delete(delete_task))
            .route("/api/v1/agents", get(list_agents).post(register_agent))
            .route("/api/v1/agents/:id", get(get_agent).delete(unregister_agent))
            .route("/api/v1/workflows", get(list_workflows).post(create_workflow))
            .route("/api/v1/workflows/:id/start", post(start_workflow))
            
            // WebSocket
            .route("/ws", get(websocket_handler))
            
            // Add middleware
            .layer(TraceLayer::new_for_http())
    }

    /// Start the gateway.
    pub async fn run(&self) -> Result<()> {
        let addr = self.config.bind_address;
        
        info!("Starting API Gateway on {}", addr);
        info!("Health check: http://{}/health", addr);
        info!("API endpoints: http://{}/api/v1/*", addr);
        info!("WebSocket: ws://{}/ws", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.router.clone()).await?;

        Ok(())
    }

    /// Route request to backend service.
    pub async fn route_request(
        &self,
        path: &str,
        method: &str,
        body: Option<Vec<u8>>,
    ) -> Result<reqwest::Response> {
        // Find matching service
        let service = self.find_service_for_path(path)?;
        
        // Build URL
        let url = format!("{}{}", service.url, path);
        
        // Make request
        let client = reqwest::Client::new();
        let response = match method {
            "GET" => client.get(&url).send().await?,
            "POST" => {
                let body = body.unwrap_or_default();
                client.post(&url).body(body).send().await?
            }
            "PUT" => {
                let body = body.unwrap_or_default();
                client.put(&url).body(body).send().await?
            }
            "DELETE" => client.delete(&url).send().await?,
            _ => return Err(anyhow::anyhow!("Unsupported method: {}", method)),
        };

        Ok(response)
    }

    /// Find service for path.
    fn find_service_for_path(&self, path: &str) -> Result<ServiceConfig> {
        // Simple routing - in production would use more sophisticated routing
        if path.starts_with("/api/v1/tasks") {
            Ok(self.config.services.iter()
                .find(|s| s.name == "task-service")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Task service not found"))?)
        } else if path.starts_with("/api/v1/agents") {
            Ok(self.config.services.iter()
                .find(|s| s.name == "agent-service")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Agent service not found"))?)
        } else if path.starts_with("/api/v1/workflows") {
            Ok(self.config.services.iter()
                .find(|s| s.name == "workflow-service")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Workflow service not found"))?)
        } else {
            Err(anyhow::anyhow!("No service found for path: {}", path))
        }
    }

    /// Get gateway statistics.
    pub async fn get_stats(&self) -> GatewayStats {
        // Would track actual metrics
        GatewayStats {
            total_requests: 0,
            active_connections: 0,
            avg_response_time_ms: 0.0,
            error_rate: 0.0,
            uptime_secs: 0,
        }
    }
}

/// Gateway statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayStats {
    pub total_requests: i64,
    pub active_connections: i64,
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_creation() {
        let config = GatewayConfig::default();
        let gateway = ApiGateway::new(config);
        
        assert!(gateway.router != Router::new());
    }

    #[tokio::test]
    async fn test_service_routing() {
        let mut config = GatewayConfig::default();
        config.services.push(ServiceConfig {
            name: "task-service".to_string(),
            url: "http://localhost:3001".to_string(),
            weight: 1,
            health_check_path: "/health".to_string(),
        });

        let gateway = ApiGateway::new(config);
        
        let service = gateway.find_service_for_path("/api/v1/tasks/123").unwrap();
        assert_eq!(service.name, "task-service");
    }
}
