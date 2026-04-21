//! GraphQL resolvers.

use async_graphql::*;
use database::{Database, TaskRepository, WorkflowRepository, AgentRepository};
use crate::types::*;

// ============ Query Resolvers ============

pub struct QueryResolver;

#[Object]
impl QueryResolver {
    /// Health check.
    async fn health(&self) -> Health {
        Health {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Get all tasks.
    async fn tasks(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Vec<Task> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        let tasks = if let Some(status) = status {
            repo.list_by_status(&status).await.unwrap_or_default()
        } else {
            repo.list().await.unwrap_or_default()
        };

        let limit = limit.unwrap_or(100) as usize;
        let offset = offset.unwrap_or(0) as usize;

        tasks
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(Task::from)
            .collect()
    }

    /// Get task by ID.
    async fn task(&self, ctx: &Context<'_>, id: String) -> Option<Task> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        match repo.get(&id).await {
            Ok(Some(task)) => Some(Task::from(task)),
            _ => None,
        }
    }

    /// Get task statistics.
    async fn task_stats(&self, ctx: &Context<'_>) -> TaskStats {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        match repo.get_stats().await {
            Ok(stats) => TaskStats::from(stats),
            Err(_) => TaskStats {
                total: 0,
                pending: 0,
                running: 0,
                completed: 0,
                failed: 0,
                cancelled: 0,
            },
        }
    }

    /// Get all workflows.
    async fn workflows(&self, ctx: &Context<'_>) -> Vec<Workflow> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = WorkflowRepository::new(pool);

        match repo.list().await {
            Ok(workflows) => workflows.into_iter().map(Workflow::from).collect(),
            Err(_) => vec![],
        }
    }

    /// Get workflow by ID.
    async fn workflow(&self, ctx: &Context<'_>, id: String) -> Option<Workflow> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = WorkflowRepository::new(pool);

        match repo.get(&id).await {
            Ok(Some(workflow)) => Some(Workflow::from(workflow)),
            _ => None,
        }
    }

    /// Get all agents.
    async fn agents(&self, ctx: &Context<'_>) -> Vec<Agent> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = AgentRepository::new(pool);

        match repo.list().await {
            Ok(agents) => agents.into_iter().map(Agent::from).collect(),
            Err(_) => vec![],
        }
    }

    /// Get agent by ID.
    async fn agent(&self, ctx: &Context<'_>, id: String) -> Option<Agent> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = AgentRepository::new(pool);

        match repo.get(&id).await {
            Ok(Some(agent)) => Some(Agent::from(agent)),
            _ => None,
        }
    }

    /// Get system metrics.
    async fn metrics(&self, ctx: &Context<'_>) -> Metrics {
        // Would fetch from Prometheus or database
        Metrics {
            total_agents: 0,
            active_agents: 0,
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            pending_tasks: 0,
            network_latency_ms: 0.0,
            message_rate: 0.0,
        }
    }
}

// ============ Mutation Resolvers ============

pub struct TaskMutation;

#[Object]
impl TaskMutation {
    /// Create a new task.
    async fn create_task(
        &self,
        ctx: &Context<'_>,
        input: CreateTaskInput,
    ) -> Result<Task, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        let mut task = database::TaskModel::default();
        task.description = input.description;
        task.priority = input.priority.unwrap_or(100);
        task.required_capabilities = input.required_capabilities.unwrap_or_default();
        task.dependencies = input.dependencies.unwrap_or_default();

        let created = repo.create(&task).await?;
        Ok(Task::from(created))
    }

    /// Update a task.
    async fn update_task(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateTaskInput,
    ) -> Result<Option<Task>, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        let mut task = match repo.get(&id).await? {
            Some(t) => t,
            None => return Ok(None),
        };

        if let Some(status) = input.status {
            task.status = status;
        }
        if let Some(agent) = input.assigned_agent {
            task.assigned_agent = Some(agent);
        }
        if let Some(priority) = input.priority {
            task.priority = priority;
        }
        if let Some(result) = input.result {
            task.result = Some(result);
        }
        if let Some(error) = input.error_message {
            task.error_message = Some(error);
        }

        let updated = repo.update(&task).await?;
        Ok(Some(Task::from(updated)))
    }

    /// Delete a task.
    async fn delete_task(&self, ctx: &Context<'_>, id: String) -> Result<bool, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        repo.delete(&id).await?;
        Ok(true)
    }

    /// Assign task to agent.
    async fn assign_task(
        &self,
        ctx: &Context<'_>,
        task_id: String,
        agent_id: String,
    ) -> Result<Option<Task>, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        let mut task = match repo.get(&task_id).await? {
            Some(t) => t,
            None => return Ok(None),
        };

        task.assigned_agent = Some(agent_id);
        task.status = "assigned".to_string();

        let updated = repo.update(&task).await?;
        Ok(Some(Task::from(updated)))
    }

    /// Complete task.
    async fn complete_task(
        &self,
        ctx: &Context<'_>,
        task_id: String,
        result: serde_json::Value,
    ) -> Result<Option<Task>, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = TaskRepository::new(pool);

        repo.complete(&task_id, &result).await?;
        
        let task = repo.get(&task_id).await?;
        Ok(task.map(Task::from))
    }
}

pub struct WorkflowMutation;

#[Object]
impl WorkflowMutation {
    /// Create a new workflow.
    async fn create_workflow(
        &self,
        ctx: &Context<'_>,
        input: CreateWorkflowInput,
    ) -> Result<Workflow, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = WorkflowRepository::new(pool);

        let mut workflow = database::WorkflowModel::default();
        workflow.name = input.name;
        workflow.description = input.description;
        workflow.version = input.version.unwrap_or_else(|| "1.0.0".to_string());
        workflow.yaml_definition = input.yaml_definition;

        let created = repo.create(&workflow).await?;
        Ok(Workflow::from(created))
    }

    /// Delete a workflow.
    async fn delete_workflow(&self, ctx: &Context<'_>, id: String) -> Result<bool, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = WorkflowRepository::new(pool);

        repo.delete(&id).await?;
        Ok(true)
    }
}

pub struct AgentMutation;

#[Object]
impl AgentMutation {
    /// Create a new agent.
    async fn create_agent(
        &self,
        ctx: &Context<'_>,
        input: CreateAgentInput,
    ) -> Result<Agent, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = AgentRepository::new(pool);

        let mut agent = database::AgentModel::default();
        agent.name = input.name;
        agent.capabilities = input.capabilities.unwrap_or_default();
        agent.metadata = input.metadata.unwrap_or_else(|| serde_json::json!({}));

        let created = repo.create(&agent).await?;
        Ok(Agent::from(created))
    }

    /// Update agent status.
    async fn update_agent_status(
        &self,
        ctx: &Context<'_>,
        id: String,
        status: String,
    ) -> Result<Option<Agent>, Error> {
        let pool = ctx.data::<sqlx::Pool>().unwrap();
        let repo = AgentRepository::new(pool);

        let mut agent = match repo.get(&id).await? {
            Some(a) => a,
            None => return Ok(None),
        };

        agent.status = status;
        agent.updated_at = chrono::Utc::now();

        let updated = repo.update(&agent).await?;
        Ok(Some(Agent::from(updated)))
    }
}

pub struct MutationResolver;

#[Object]
impl MutationResolver {
    /// Placeholder for additional mutations.
    async fn custom_mutation(&self) -> String {
        "Custom mutation".to_string()
    }
}
