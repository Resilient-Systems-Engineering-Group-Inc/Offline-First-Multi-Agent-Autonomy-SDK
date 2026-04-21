//! GraphQL types.

use async_graphql::*;
use chrono::{DateTime, Utc};
use database::models::*;

// ============ Task Types ============

#[derive(InputObject, Clone)]
pub struct CreateTaskInput {
    pub description: String,
    pub priority: Option<i32>,
    pub required_capabilities: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub workflow_instance_id: Option<String>,
}

#[derive(InputObject, Clone)]
pub struct UpdateTaskInput {
    pub status: Option<String>,
    pub assigned_agent: Option<String>,
    pub priority: Option<i32>,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub assigned_agent: Option<String>,
    pub workflow_instance_id: Option<String>,
    pub parameters: serde_json::Value,
    pub required_capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub retry_count: i32,
}

impl From<TaskModel> for Task {
    fn from(model: TaskModel) -> Self {
        Self {
            id: model.id,
            description: model.description,
            status: model.status,
            priority: model.priority,
            created_at: model.created_at,
            updated_at: model.updated_at,
            started_at: model.started_at,
            completed_at: model.completed_at,
            assigned_agent: model.assigned_agent,
            workflow_instance_id: model.workflow_instance_id,
            parameters: model.parameters,
            required_capabilities: model.required_capabilities,
            dependencies: model.dependencies,
            result: model.result,
            error_message: model.error_message,
            retry_count: model.retry_count,
        }
    }
}

// ============ Workflow Types ============

#[derive(InputObject, Clone)]
pub struct CreateWorkflowInput {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub yaml_definition: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub yaml_definition: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
}

impl From<WorkflowModel> for Workflow {
    fn from(model: WorkflowModel) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            version: model.version,
            yaml_definition: model.yaml_definition,
            created_at: model.created_at,
            updated_at: model.updated_at,
            is_active: model.is_active,
            metadata: model.metadata,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WorkflowInstance {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub progress: f64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parameters: serde_json::Value,
    pub output: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<WorkflowInstanceModel> for WorkflowInstance {
    fn from(model: WorkflowInstanceModel) -> Self {
        Self {
            id: model.id,
            workflow_id: model.workflow_id,
            status: model.status,
            progress: model.progress,
            started_at: model.started_at,
            completed_at: model.completed_at,
            parameters: model.parameters,
            output: model.output,
            error_message: model.error_message,
            created_at: model.created_at,
        }
    }
}

// ============ Agent Types ============

#[derive(InputObject, Clone)]
pub struct CreateAgentInput {
    pub name: String,
    pub capabilities: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(SimpleObject, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub resources: serde_json::Value,
    pub connected_peers: i32,
    pub active_tasks: Vec<String>,
    pub last_heartbeat: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl From<AgentModel> for Agent {
    fn from(model: AgentModel) -> Self {
        Self {
            id: model.id,
            name: model.name,
            status: model.status,
            capabilities: model.capabilities,
            resources: model.resources,
            connected_peers: model.connected_peers,
            active_tasks: model.active_tasks,
            last_heartbeat: model.last_heartbeat,
            created_at: model.created_at,
            updated_at: model.updated_at,
            metadata: model.metadata,
        }
    }
}

// ============ Statistics Types ============

#[derive(SimpleObject, Clone)]
pub struct TaskStats {
    pub total: i64,
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
}

impl From<database::TaskStats> for TaskStats {
    fn from(stats: database::TaskStats) -> Self {
        Self {
            total: stats.total,
            pending: stats.pending,
            running: stats.running,
            completed: stats.completed,
            failed: stats.failed,
            cancelled: stats.cancelled,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct WorkflowStats {
    pub total: i64,
    pub active: i64,
    pub completed: i64,
    pub failed: i64,
}

// ============ System Types ============

#[derive(SimpleObject, Clone)]
pub struct Health {
    pub status: String,
    pub version: String,
    pub timestamp: i64,
}

#[derive(SimpleObject, Clone)]
pub struct Metrics {
    pub total_agents: i64,
    pub active_agents: i64,
    pub total_tasks: i64,
    pub completed_tasks: i64,
    pub failed_tasks: i64,
    pub pending_tasks: i64,
    pub network_latency_ms: f64,
    pub message_rate: f64,
}

// ============ Connection Types (for pagination) ============

#[derive(SimpleObject, Clone)]
pub struct TaskConnection {
    pub edges: Vec<TaskEdge>,
    pub page_info: PageInfo,
}

#[derive(SimpleObject, Clone)]
pub struct TaskEdge {
    pub node: Task,
    pub cursor: String,
}

#[derive(SimpleObject, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}
