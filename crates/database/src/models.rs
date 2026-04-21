//! Database models and entities.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ============ Task Models ============

/// Task entity in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskModel {
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

impl Default for TaskModel {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            description: String::new(),
            status: "pending".to_string(),
            priority: 100,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            assigned_agent: None,
            workflow_instance_id: None,
            parameters: serde_json::json!({}),
            required_capabilities: vec![],
            dependencies: vec![],
            result: None,
            error_message: None,
            retry_count: 0,
        }
    }
}

// ============ Workflow Models ============

/// Workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowModel {
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

impl Default for WorkflowModel {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            description: None,
            version: "1.0.0".to_string(),
            yaml_definition: None,
            created_at: now,
            updated_at: now,
            is_active: true,
            metadata: serde_json::json!({}),
        }
    }
}

/// Workflow instance (running/completed workflow).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowInstanceModel {
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

impl Default for WorkflowInstanceModel {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            workflow_id: String::new(),
            status: "pending".to_string(),
            progress: 0.0,
            started_at: now,
            completed_at: None,
            parameters: serde_json::json!({}),
            output: serde_json::json!({}),
            error_message: None,
            created_at: now,
        }
    }
}

// ============ Agent Models ============

/// Agent entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentModel {
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

impl Default for AgentModel {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            status: "offline".to_string(),
            capabilities: vec![],
            resources: serde_json::json!({}),
            connected_peers: 0,
            active_tasks: vec![],
            last_heartbeat: now,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        }
    }
}

// ============ Audit Log Models ============

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogModel {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
}

impl Default for AuditLogModel {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor_id: None,
            action: String::new(),
            entity_type: String::new(),
            entity_id: String::new(),
            old_value: None,
            new_value: None,
            metadata: serde_json::json!({}),
            ip_address: None,
        }
    }
}

// ============ Authentication Models ============

/// User account.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserModel {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

/// API token.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiTokenModel {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

// ============ Query Results ============

/// Task statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: i64,
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
}

/// Workflow statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStats {
    pub total: i64,
    pub active: i64,
    pub completed: i64,
    pub failed: i64,
}

/// Time series data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}
