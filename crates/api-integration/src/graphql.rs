//! GraphQL client and server integration for multi-agent systems.
//!
//! This module provides GraphQL-based API for querying and mutating
//! agent state, tasks, and other system resources.

use std::sync::Arc;

use async_graphql::{
    Context, EmptyMutation, EmptySubscription, FieldResult, Object, Schema, SimpleObject,
    Subscription,
};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use futures::Stream;
use warp::Filter;

use crate::error::{ApiIntegrationError, Result};

/// GraphQL schema root containing all queries and mutations.
pub type AgentSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

/// Query root for GraphQL API.
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get information about a specific agent.
    async fn agent(&self, ctx: &Context<'_>, id: u64) -> FieldResult<Agent> {
        let registry = ctx.data::<Arc<dyn AgentRegistry>>()?;
        match registry.get_agent(id).await {
            Some(agent) => Ok(agent),
            None => Err("Agent not found".into()),
        }
    }

    /// List all agents in the system.
    async fn agents(&self, ctx: &Context<'_>) -> FieldResult<Vec<Agent>> {
        let registry = ctx.data::<Arc<dyn AgentRegistry>>()?;
        Ok(registry.list_agents().await)
    }

    /// Get tasks assigned to a specific agent.
    async fn tasks(&self, ctx: &Context<'_>, agent_id: Option<u64>) -> FieldResult<Vec<Task>> {
        let task_store = ctx.data::<Arc<dyn TaskStore>>()?;
        Ok(task_store.get_tasks(agent_id).await)
    }

    /// Health check endpoint.
    async fn health(&self) -> FieldResult<HealthStatus> {
        Ok(HealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

/// Agent information exposed via GraphQL.
#[derive(SimpleObject, Clone)]
pub struct Agent {
    /// Unique agent identifier.
    pub id: u64,
    /// Current status of the agent.
    pub status: String,
    /// Agent capabilities.
    pub capabilities: Vec<String>,
    /// Resource usage metrics.
    pub resources: ResourceMetrics,
    /// Timestamp of last heartbeat.
    pub last_seen: i64,
}

/// Resource metrics for an agent.
#[derive(SimpleObject, Clone)]
pub struct ResourceMetrics {
    /// CPU usage percentage (0-100).
    pub cpu_usage: f32,
    /// Memory usage in bytes.
    pub memory_usage: u64,
    /// Battery level percentage (0-100) if applicable.
    pub battery_level: Option<f32>,
    /// Network latency in milliseconds.
    pub network_latency: Option<f32>,
}

/// Task representation in GraphQL.
#[derive(SimpleObject, Clone)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// Task description.
    pub description: String,
    /// Agent ID this task is assigned to (if any).
    pub assigned_to: Option<u64>,
    /// Task status.
    pub status: String,
    /// Estimated duration in seconds.
    pub estimated_duration: u64,
    /// Deadline timestamp (Unix epoch).
    pub deadline: Option<i64>,
}

/// Health status response.
#[derive(SimpleObject)]
pub struct HealthStatus {
    /// Status string ("healthy", "degraded", "unhealthy").
    pub status: String,
    /// Timestamp of the check.
    pub timestamp: i64,
    /// Version of the API.
    pub version: String,
}

/// Trait for accessing agent data (abstraction over real implementation).
#[async_trait::async_trait]
pub trait AgentRegistry: Send + Sync {
    /// Get agent by ID.
    async fn get_agent(&self, id: u64) -> Option<Agent>;
    /// List all agents.
    async fn list_agents(&self) -> Vec<Agent>;
}

/// Trait for accessing task data.
#[async_trait::async_trait]
pub trait TaskStore: Send + Sync {
    /// Get tasks, optionally filtered by agent ID.
    async fn get_tasks(&self, agent_id: Option<u64>) -> Vec<Task>;
}

/// GraphQL server for exposing agent system state.
pub struct GraphQLServer {
    addr: std::net::SocketAddr,
    schema: AgentSchema,
}

impl GraphQLServer {
    /// Create a new GraphQL server with the given data providers.
    pub fn new(
        addr: std::net::SocketAddr,
        agent_registry: Arc<dyn AgentRegistry>,
        task_store: Arc<dyn TaskStore>,
    ) -> Self {
        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
            .data(agent_registry)
            .data(task_store)
            .finish();

        Self { addr, schema }
    }

    /// Start the GraphQL server.
    ///
    /// This runs indefinitely until the server is shut down.
    pub async fn serve(self) -> Result<()> {
        let schema = self.schema;

        // GraphQL endpoint
        let graphql_post = async_graphql_warp::graphql(schema).and_then(
            |(schema, request): (AgentSchema, async_graphql::Request)| async move {
                Ok::<_, std::convert::Infallible>(GraphQLResponse::from(schema.execute(request).await))
            },
        );

        // GraphiQL playground for development
        let graphiql = warp::path!("graphiql").and(warp::get()).map(|| {
            warp::reply::html(async_graphql::http::GraphiQLSource::build().endpoint("/graphql").finish())
        });

        // Health endpoint
        let health = warp::path!("health").and(warp::get()).map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "ok",
                "timestamp": chrono::Utc::now().timestamp(),
            }))
        });

        let routes = warp::post()
            .and(warp::path("graphql"))
            .and(graphql_post)
            .or(graphiql)
            .or(health)
            .with(warp::cors().allow_any_origin());

        tracing::info!("Starting GraphQL server on {}", self.addr);
        warp::serve(routes).run(self.addr).await;

        Ok(())
    }
}

/// GraphQL client for querying external GraphQL APIs.
pub struct GraphQLClient {
    endpoint: String,
    client: reqwest::Client,
}

impl GraphQLClient {
    /// Create a new GraphQL client.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Execute a GraphQL query.
    pub async fn query(&self, query: &str, variables: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let mut request = self.client.post(&self.endpoint);
        
        let body = serde_json::json!({
            "query": query,
            "variables": variables.unwrap_or(serde_json::Value::Null),
        });

        request = request.json(&body);

        let response = request
            .send()
            .await
            .map_err(|e| ApiIntegrationError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ApiIntegrationError::RequestError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ApiIntegrationError::ParseError(e.to_string()))?;

        // Check for GraphQL errors
        if let Some(errors) = json.get("errors") {
            if errors.is_array() && !errors.as_array().unwrap().is_empty() {
                return Err(ApiIntegrationError::RequestError(format!(
                    "GraphQL errors: {}",
                    errors
                )));
            }
        }

        Ok(json)
    }

    /// Execute a GraphQL mutation.
    pub async fn mutate(&self, mutation: &str, variables: Option<serde_json::Value>) -> Result<serde_json::Value> {
        self.query(mutation, variables).await
    }
}

/// Integration with mesh transport: expose mesh data via GraphQL.
pub struct GraphQLMeshIntegration {
    /// GraphQL schema
    schema: AgentSchema,
    /// Mesh transport for real-time updates
    mesh_transport: Arc<dyn crate::mesh_transport::Transport>,
}

impl GraphQLMeshIntegration {
    /// Create a new integration between GraphQL and mesh transport.
    pub fn new(
        agent_registry: Arc<dyn AgentRegistry>,
        task_store: Arc<dyn TaskStore>,
        mesh_transport: Arc<dyn crate::mesh_transport::Transport>,
    ) -> Self {
        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
            .data(agent_registry)
            .data(task_store)
            .data(mesh_transport)
            .finish();

        Self {
            schema,
            mesh_transport,
        }
    }

    /// Get the GraphQL schema for use in a server.
    pub fn schema(&self) -> AgentSchema {
        self.schema.clone()
    }

    /// Execute a GraphQL query against the integrated schema.
    pub async fn execute_query(&self, query: &str) -> Result<async_graphql::Response> {
        let request = async_graphql::Request::new(query);
        let response = self.schema.execute(request).await;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockAgentRegistry {
        agents: Mutex<Vec<Agent>>,
    }

    impl MockAgentRegistry {
        fn new() -> Self {
            Self {
                agents: Mutex::new(vec![
                    Agent {
                        id: 1,
                        status: "running".to_string(),
                        capabilities: vec!["compute".to_string(), "storage".to_string()],
                        resources: ResourceMetrics {
                            cpu_usage: 25.5,
                            memory_usage: 1024 * 1024 * 512, // 512 MB
                            battery_level: Some(85.0),
                            network_latency: Some(12.5),
                        },
                        last_seen: chrono::Utc::now().timestamp(),
                    },
                    Agent {
                        id: 2,
                        status: "idle".to_string(),
                        capabilities: vec!["sensor".to_string()],
                        resources: ResourceMetrics {
                            cpu_usage: 5.0,
                            memory_usage: 1024 * 1024 * 256, // 256 MB
                            battery_level: Some(45.0),
                            network_latency: Some(8.2),
                        },
                        last_seen: chrono::Utc::now().timestamp(),
                    },
                ]),
            }
        }
    }

    #[async_trait::async_trait]
    impl AgentRegistry for MockAgentRegistry {
        async fn get_agent(&self, id: u64) -> Option<Agent> {
            self.agents.lock().unwrap().iter().find(|a| a.id == id).cloned()
        }

        async fn list_agents(&self) -> Vec<Agent> {
            self.agents.lock().unwrap().clone()
        }
    }

    struct MockTaskStore;

    #[async_trait::async_trait]
    impl TaskStore for MockTaskStore {
        async fn get_tasks(&self, _agent_id: Option<u64>) -> Vec<Task> {
            vec![
                Task {
                    id: "task-1".to_string(),
                    description: "Process sensor data".to_string(),
                    assigned_to: Some(1),
                    status: "in_progress".to_string(),
                    estimated_duration: 60,
                    deadline: Some(chrono::Utc::now().timestamp() + 3600),
                },
                Task {
                    id: "task-2".to_string(),
                    description: "Backup logs".to_string(),
                    assigned_to: Some(2),
                    status: "pending".to_string(),
                    estimated_duration: 120,
                    deadline: None,
                },
            ]
        }
    }

    #[tokio::test]
    async fn test_graphql_schema_creation() {
        let registry = Arc::new(MockAgentRegistry::new());
        let task_store = Arc::new(MockTaskStore);
        
        let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
            .data(registry)
            .data(task_store)
            .finish();

        // Execute a simple query
        let query = r#"
            query {
                health {
                    status
                    version
                }
            }
        "#;
        
        let response = schema.execute(query).await;
        assert!(response.is_ok());
        let data = response.data;
        assert!(data.is_object());
    }

    #[tokio::test]
    async fn test_graphql_client_creation() {
        let client = GraphQLClient::new("http://localhost:8080/graphql");
        assert_eq!(client.endpoint, "http://localhost:8080/graphql");
    }
}