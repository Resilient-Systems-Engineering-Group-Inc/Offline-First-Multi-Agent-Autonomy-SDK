//! Query handlers for CQRS.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Query trait.
pub trait Query: Send + Sync {
    fn query_type(&self) -> &str;
}

/// Query handler trait.
#[async_trait::async_trait]
pub trait QueryHandler<Q: Query, R: Send + Sync>: Send + Sync {
    async fn handle(&self, query: Q) -> Result<R>;
}

/// Get task by ID query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskByIdQuery {
    pub task_id: String,
}

impl Query for GetTaskByIdQuery {
    fn query_type(&self) -> &str {
        "GetTaskById"
    }
}

/// Task read model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReadModel {
    pub id: String,
    pub description: String,
    pub priority: i32,
    pub status: String,
    pub assigned_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// List tasks query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<String>,
    pub assigned_agent: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

impl Query for ListTasksQuery {
    fn query_type(&self) -> &str {
        "ListTasks"
    }
}

/// Get agent by ID query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAgentByIdQuery {
    pub agent_id: String,
}

impl Query for GetAgentByIdQuery {
    fn query_type(&self) -> &str {
        "GetAgentById"
    }
}

/// Agent read model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReadModel {
    pub id: String,
    pub name: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub current_task: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Query bus for dispatching queries.
pub struct QueryBus {
    handlers: std::collections::HashMap<String, Box<dyn QueryHandlerBase>>,
}

trait QueryHandlerBase: Send + Sync {
    fn handle(&self, query: serde_json::Value) -> Result<serde_json::Value>;
}

impl QueryBus {
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    pub fn register<Q, R, H>(&mut self, query_type: &str, handler: H)
    where
        Q: Query + for<'de> Deserialize<'de>,
        R: Serialize,
        H: QueryHandler<Q, R> + 'static,
    {
        struct Wrapper<Q, R, H> {
            handler: H,
            _phantom: std::marker::PhantomData<(Q, R)>,
        }

        impl<Q, R, H> QueryHandlerBase for Wrapper<Q, R, H>
        where
            Q: Query + for<'de> Deserialize<'de>,
            R: Serialize,
            H: QueryHandler<Q, R>,
        {
            fn handle(&self, query: serde_json::Value) -> Result<serde_json::Value> {
                let query: Q = serde_json::from_value(query)?;
                let result = futures::executor::block_on(self.handler.handle(query))?;
                Ok(serde_json::to_value(result)?)
            }
        }

        self.handlers.insert(
            query_type.to_string(),
            Box::new(Wrapper {
                handler,
                _phantom: std::marker::PhantomData,
            }),
        );
    }

    pub async fn dispatch<R: for<'de> Deserialize<'de>>(&self, query_type: &str, query: serde_json::Value) -> Result<R> {
        let handler = self.handlers.get(query_type)
            .ok_or_else(|| anyhow::anyhow!("No handler for query type: {}", query_type))?;

        let result = handler.handle(query)?;
        Ok(serde_json::from_value(result)?)
    }
}

impl Default for QueryBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queries() {
        let get_task = GetTaskByIdQuery {
            task_id: "task-1".to_string(),
        };

        assert_eq!(get_task.query_type(), "GetTaskById");
    }
}
