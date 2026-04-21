//! Database repositories for CRUD operations.

use sqlx::{Pool, Row};
use crate::models::*;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};
use chrono::Utc;

/// Base repository trait.
#[async_trait::async_trait]
pub trait Repository<T> {
    async fn create(&self, item: &T) -> Result<T>;
    async fn get(&self, id: &str) -> Result<Option<T>>;
    async fn update(&self, item: &T) -> Result<T>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn list(&self) -> Result<Vec<T>>;
}

// ============ Task Repository ============

pub struct TaskRepository<'a> {
    pool: &'a Pool,
}

impl<'a> TaskRepository<'a> {
    pub fn new(pool: &'a Pool) -> Self {
        Self { pool }
    }

    /// Create a new task.
    pub async fn create(&self, task: &TaskModel) -> Result<TaskModel> {
        let query = r#"
            INSERT INTO tasks (
                id, description, status, priority, created_at, updated_at,
                started_at, completed_at, assigned_agent, workflow_instance_id,
                parameters, required_capabilities, dependencies, result,
                error_message, retry_count
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&task.id)
            .bind(&task.description)
            .bind(&task.status)
            .bind(task.priority)
            .bind(task.created_at)
            .bind(task.updated_at)
            .bind(task.started_at)
            .bind(task.completed_at)
            .bind(&task.assigned_agent)
            .bind(&task.workflow_instance_id)
            .bind(serde_json::to_string(&task.parameters)?)
            .bind(serde_json::to_string(&task.required_capabilities)?)
            .bind(serde_json::to_string(&task.dependencies)?)
            .bind(task.result.as_ref().map(|v| serde_json::to_string(v)).transpose()?)
            .bind(&task.error_message)
            .bind(task.retry_count)
            .execute(self.pool)
            .await?;

        Ok(task.clone())
    }

    /// Get task by ID.
    pub async fn get(&self, id: &str) -> Result<Option<TaskModel>> {
        let query = "SELECT * FROM tasks WHERE id = ?";

        match sqlx::query_as::<_, TaskModel>(query)
            .bind(id)
            .fetch_optional(self.pool)
            .await?
        {
            Some(task) => Ok(Some(task)),
            None => Ok(None),
        }
    }

    /// Update task.
    pub async fn update(&self, task: &TaskModel) -> Result<TaskModel> {
        let query = r#"
            UPDATE tasks SET
                description = ?, status = ?, priority = ?, updated_at = ?,
                started_at = ?, completed_at = ?, assigned_agent = ?,
                workflow_instance_id = ?, parameters = ?, required_capabilities = ?,
                dependencies = ?, result = ?, error_message = ?, retry_count = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(&task.description)
            .bind(&task.status)
            .bind(task.priority)
            .bind(task.updated_at)
            .bind(task.started_at)
            .bind(task.completed_at)
            .bind(&task.assigned_agent)
            .bind(&task.workflow_instance_id)
            .bind(serde_json::to_string(&task.parameters)?)
            .bind(serde_json::to_string(&task.required_capabilities)?)
            .bind(serde_json::to_string(&task.dependencies)?)
            .bind(task.result.as_ref().map(|v| serde_json::to_string(v)).transpose()?)
            .bind(&task.error_message)
            .bind(task.retry_count)
            .bind(&task.id)
            .execute(self.pool)
            .await?;

        Ok(task.clone())
    }

    /// Delete task.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let query = "DELETE FROM tasks WHERE id = ?";
        
        sqlx::query(query)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// List all tasks.
    pub async fn list(&self) -> Result<Vec<TaskModel>> {
        let query = "SELECT * FROM tasks ORDER BY created_at DESC";
        
        sqlx::query_as::<_, TaskModel>(query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list tasks: {}", e))
    }

    /// List tasks by status.
    pub async fn list_by_status(&self, status: &str) -> Result<Vec<TaskModel>> {
        let query = "SELECT * FROM tasks WHERE status = ? ORDER BY created_at DESC";
        
        sqlx::query_as::<_, TaskModel>(query)
            .bind(status)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list tasks: {}", e))
    }

    /// List tasks by workflow instance.
    pub async fn list_by_workflow(&self, workflow_instance_id: &str) -> Result<Vec<TaskModel>> {
        let query = "SELECT * FROM tasks WHERE workflow_instance_id = ? ORDER BY created_at ASC";
        
        sqlx::query_as::<_, TaskModel>(query)
            .bind(workflow_instance_id)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list tasks: {}", e))
    }

    /// Get task statistics.
    pub async fn get_stats(&self) -> Result<TaskStats> {
        let query = r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END) as running,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN status = 'cancelled' THEN 1 ELSE 0 END) as cancelled
            FROM tasks
        "#;

        let row = sqlx::query(query).fetch_one(self.pool).await?;

        Ok(TaskStats {
            total: row.get("total"),
            pending: row.get("pending"),
            running: row.get("running"),
            completed: row.get("completed"),
            failed: row.get("failed"),
            cancelled: row.get("cancelled"),
        })
    }

    /// Update task status.
    pub async fn update_status(&self, id: &str, status: &str) -> Result<()> {
        let query = "UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?";
        
        sqlx::query(query)
            .bind(status)
            .bind(Utc::now())
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Complete task with result.
    pub async fn complete(&self, id: &str, result: &serde_json::Value) -> Result<()> {
        let query = r#"
            UPDATE tasks SET
                status = 'completed', completed_at = ?, updated_at = ?, result = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(Utc::now())
            .bind(Utc::now())
            .bind(serde_json::to_string(result)?)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Fail task with error.
    pub async fn fail(&self, id: &str, error: &str) -> Result<()> {
        let query = r#"
            UPDATE tasks SET
                status = 'failed', completed_at = ?, updated_at = ?, error_message = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(Utc::now())
            .bind(Utc::now())
            .bind(error)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}

// ============ Workflow Repository ============

pub struct WorkflowRepository<'a> {
    pool: &'a Pool,
}

impl<'a> WorkflowRepository<'a> {
    pub fn new(pool: &'a Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, workflow: &WorkflowModel) -> Result<WorkflowModel> {
        let query = r#"
            INSERT INTO workflows (
                id, name, description, version, yaml_definition,
                created_at, updated_at, is_active, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&workflow.id)
            .bind(&workflow.name)
            .bind(&workflow.description)
            .bind(&workflow.version)
            .bind(&workflow.yaml_definition)
            .bind(workflow.created_at)
            .bind(workflow.updated_at)
            .bind(workflow.is_active)
            .bind(serde_json::to_string(&workflow.metadata)?)
            .execute(self.pool)
            .await?;

        Ok(workflow.clone())
    }

    pub async fn get(&self, id: &str) -> Result<Option<WorkflowModel>> {
        let query = "SELECT * FROM workflows WHERE id = ?";
        
        sqlx::query_as::<_, WorkflowModel>(query)
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get workflow: {}", e))
    }

    pub async fn update(&self, workflow: &WorkflowModel) -> Result<WorkflowModel> {
        let query = r#"
            UPDATE workflows SET
                name = ?, description = ?, version = ?, yaml_definition = ?,
                updated_at = ?, is_active = ?, metadata = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(&workflow.name)
            .bind(&workflow.description)
            .bind(&workflow.version)
            .bind(&workflow.yaml_definition)
            .bind(workflow.updated_at)
            .bind(workflow.is_active)
            .bind(serde_json::to_string(&workflow.metadata)?)
            .bind(&workflow.id)
            .execute(self.pool)
            .await?;

        Ok(workflow.clone())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let query = "DELETE FROM workflows WHERE id = ?";
        
        sqlx::query(query)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<WorkflowModel>> {
        let query = "SELECT * FROM workflows WHERE is_active = 1 ORDER BY created_at DESC";
        
        sqlx::query_as::<_, WorkflowModel>(query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list workflows: {}", e))
    }

    pub async fn get_stats(&self) -> Result<WorkflowStats> {
        let query = r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status IN ('pending', 'running', 'paused') THEN 1 ELSE 0 END) as active,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
            FROM workflow_instances
        "#;

        let row = sqlx::query(query).fetch_one(self.pool).await?;

        Ok(WorkflowStats {
            total: row.get("total"),
            active: row.get("active"),
            completed: row.get("completed"),
            failed: row.get("failed"),
        })
    }
}

// ============ Workflow Instance Repository ============

pub struct WorkflowInstanceRepository<'a> {
    pool: &'a Pool,
}

impl<'a> WorkflowInstanceRepository<'a> {
    pub fn new(pool: &'a Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, instance: &WorkflowInstanceModel) -> Result<WorkflowInstanceModel> {
        let query = r#"
            INSERT INTO workflow_instances (
                id, workflow_id, status, progress, started_at, completed_at,
                parameters, output, error_message, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&instance.id)
            .bind(&instance.workflow_id)
            .bind(&instance.status)
            .bind(instance.progress)
            .bind(instance.started_at)
            .bind(instance.completed_at)
            .bind(serde_json::to_string(&instance.parameters)?)
            .bind(serde_json::to_string(&instance.output)?)
            .bind(&instance.error_message)
            .bind(instance.created_at)
            .execute(self.pool)
            .await?;

        Ok(instance.clone())
    }

    pub async fn get(&self, id: &str) -> Result<Option<WorkflowInstanceModel>> {
        let query = "SELECT * FROM workflow_instances WHERE id = ?";
        
        sqlx::query_as::<_, WorkflowInstanceModel>(query)
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get workflow instance: {}", e))
    }

    pub async fn update(&self, instance: &WorkflowInstanceModel) -> Result<WorkflowInstanceModel> {
        let query = r#"
            UPDATE workflow_instances SET
                status = ?, progress = ?, completed_at = ?, output = ?,
                error_message = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(&instance.status)
            .bind(instance.progress)
            .bind(instance.completed_at)
            .bind(serde_json::to_string(&instance.output)?)
            .bind(&instance.error_message)
            .bind(&instance.id)
            .execute(self.pool)
            .await?;

        Ok(instance.clone())
    }

    pub async fn list_active(&self) -> Result<Vec<WorkflowInstanceModel>> {
        let query = r#"
            SELECT * FROM workflow_instances 
            WHERE status IN ('pending', 'running', 'paused')
            ORDER BY started_at DESC
        "#;

        sqlx::query_as::<_, WorkflowInstanceModel>(query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list workflow instances: {}", e))
    }
}

// ============ Agent Repository ============

pub struct AgentRepository<'a> {
    pool: &'a Pool,
}

impl<'a> AgentRepository<'a> {
    pub fn new(pool: &'a Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, agent: &AgentModel) -> Result<AgentModel> {
        let query = r#"
            INSERT INTO agents (
                id, name, status, capabilities, resources, connected_peers,
                active_tasks, last_heartbeat, created_at, updated_at, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&agent.id)
            .bind(&agent.name)
            .bind(&agent.status)
            .bind(serde_json::to_string(&agent.capabilities)?)
            .bind(serde_json::to_string(&agent.resources)?)
            .bind(agent.connected_peers)
            .bind(serde_json::to_string(&agent.active_tasks)?)
            .bind(agent.last_heartbeat)
            .bind(agent.created_at)
            .bind(agent.updated_at)
            .bind(serde_json::to_string(&agent.metadata)?)
            .execute(self.pool)
            .await?;

        Ok(agent.clone())
    }

    pub async fn get(&self, id: &str) -> Result<Option<AgentModel>> {
        let query = "SELECT * FROM agents WHERE id = ?";
        
        sqlx::query_as::<_, AgentModel>(query)
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get agent: {}", e))
    }

    pub async fn update(&self, agent: &AgentModel) -> Result<AgentModel> {
        let query = r#"
            UPDATE agents SET
                name = ?, status = ?, capabilities = ?, resources = ?,
                connected_peers = ?, active_tasks = ?, last_heartbeat = ?,
                updated_at = ?, metadata = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(&agent.name)
            .bind(&agent.status)
            .bind(serde_json::to_string(&agent.capabilities)?)
            .bind(serde_json::to_string(&agent.resources)?)
            .bind(agent.connected_peers)
            .bind(serde_json::to_string(&agent.active_tasks)?)
            .bind(agent.last_heartbeat)
            .bind(agent.updated_at)
            .bind(serde_json::to_string(&agent.metadata)?)
            .bind(&agent.id)
            .execute(self.pool)
            .await?;

        Ok(agent.clone())
    }

    pub async fn list(&self) -> Result<Vec<AgentModel>> {
        let query = "SELECT * FROM agents ORDER BY last_heartbeat DESC";
        
        sqlx::query_as::<_, AgentModel>(query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list agents: {}", e))
    }

    pub async fn list_active(&self) -> Result<Vec<AgentModel>> {
        let query = "SELECT * FROM agents WHERE status = 'active' ORDER BY last_heartbeat DESC";
        
        sqlx::query_as::<_, AgentModel>(query)
            .bind("active")
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list agents: {}", e))
    }
}

// ============ Audit Log Repository ============

pub struct AuditLogRepository<'a> {
    pool: &'a Pool,
}

impl<'a> AuditLogRepository<'a> {
    pub fn new(pool: &'a Pool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, log: &AuditLogModel) -> Result<AuditLogModel> {
        let query = r#"
            INSERT INTO audit_logs (
                id, timestamp, actor_id, action, entity_type, entity_id,
                old_value, new_value, metadata, ip_address
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&log.id)
            .bind(log.timestamp)
            .bind(&log.actor_id)
            .bind(&log.action)
            .bind(&log.entity_type)
            .bind(&log.entity_id)
            .bind(log.old_value.as_ref().map(|v| serde_json::to_string(v)).transpose()?)
            .bind(log.new_value.as_ref().map(|v| serde_json::to_string(v)).transpose()?)
            .bind(serde_json::to_string(&log.metadata)?)
            .bind(&log.ip_address)
            .execute(self.pool)
            .await?;

        Ok(log.clone())
    }

    pub async fn list_for_entity(&self, entity_type: &str, entity_id: &str, limit: i64) -> Result<Vec<AuditLogModel>> {
        let query = r#"
            SELECT * FROM audit_logs 
            WHERE entity_type = ? AND entity_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
        "#;

        sqlx::query_as::<_, AuditLogModel>(query)
            .bind(entity_type)
            .bind(entity_id)
            .bind(limit)
            .fetch_all(self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list audit logs: {}", e))
    }
}
