//! Projections for building read models.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Projection trait.
#[async_trait::async_trait]
pub trait Projection: Send + Sync {
    /// Projection name.
    fn name(&self) -> &str;

    /// Check if projection is interested in event type.
    fn is_interested_in(&self, event_type: &str) -> bool;

    /// Handle event.
    async fn handle(&self, event: &crate::event::DomainEvent) -> Result<()>;

    /// Reset projection.
    async fn reset(&self) -> Result<()>;

    /// Get projection position (last processed event position).
    async fn get_position(&self) -> Result<i64>;
}

/// Task projection - builds task read model.
pub struct TaskProjection {
    name: String,
    position: std::sync::atomic::AtomicI64,
}

impl TaskProjection {
    pub fn new() -> Self {
        Self {
            name: "tasks".to_string(),
            position: std::sync::atomic::AtomicI64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Projection for TaskProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_interested_in(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            "TaskCreated" | "TaskAssigned" | "TaskStarted" | "TaskCompleted" | "TaskFailed"
        )
    }

    async fn handle(&self, event: &crate::event::DomainEvent) -> Result<()> {
        // Would update read model database here
        self.position.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn reset(&self) -> Result<()> {
        self.position.store(0, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn get_position(&self) -> Result<i64> {
        Ok(self.position.load(std::sync::atomic::Ordering::SeqCst))
    }
}

impl Default for TaskProjection {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent projection - builds agent read model.
pub struct AgentProjection {
    name: String,
    position: std::sync::atomic::AtomicI64,
}

impl AgentProjection {
    pub fn new() -> Self {
        Self {
            name: "agents".to_string(),
            position: std::sync::atomic::AtomicI64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Projection for AgentProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_interested_in(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            "AgentRegistered" | "AgentConnected" | "AgentDisconnected" | "AgentStatusChanged"
        )
    }

    async fn handle(&self, event: &crate::event::DomainEvent) -> Result<()> {
        self.position.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn reset(&self) -> Result<()> {
        self.position.store(0, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn get_position(&self) -> Result<i64> {
        Ok(self.position.load(std::sync::atomic::Ordering::SeqCst))
    }
}

impl Default for AgentProjection {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics projection - builds metrics for dashboards.
pub struct MetricsProjection {
    name: String,
    metrics: RwLock<ProjectionMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectionMetrics {
    pub total_tasks_created: i64,
    pub total_tasks_completed: i64,
    pub total_tasks_failed: i64,
    pub total_agents_registered: i64,
    pub avg_task_duration_secs: f64,
}

use tokio::sync::RwLock;

impl MetricsProjection {
    pub fn new() -> Self {
        Self {
            name: "metrics".to_string(),
            metrics: RwLock::new(ProjectionMetrics::default()),
        }
    }

    pub async fn get_metrics(&self) -> ProjectionMetrics {
        self.metrics.read().await.clone()
    }
}

#[async_trait::async_trait]
impl Projection for MetricsProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_interested_in(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            "TaskCreated" | "TaskCompleted" | "TaskFailed" | "AgentRegistered"
        )
    }

    async fn handle(&self, event: &crate::event::DomainEvent) -> Result<()> {
        let mut metrics = self.metrics.write().await;

        match event.event_type.as_str() {
            "TaskCreated" => metrics.total_tasks_created += 1,
            "TaskCompleted" => metrics.total_tasks_completed += 1,
            "TaskFailed" => metrics.total_tasks_failed += 1,
            "AgentRegistered" => metrics.total_agents_registered += 1,
            _ => {}
        }

        Ok(())
    }

    async fn reset(&self) -> Result<()> {
        *self.metrics.write().await = ProjectionMetrics::default();
        Ok(())
    }

    async fn get_position(&self) -> Result<i64> {
        let metrics = self.metrics.read().await;
        Ok(metrics.total_tasks_created + metrics.total_agents_registered)
    }
}

impl Default for MetricsProjection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::DomainEvent;

    #[tokio::test]
    async fn test_task_projection() {
        let projection = TaskProjection::new();

        assert!(projection.is_interested_in("TaskCreated"));
        assert!(!projection.is_interested_in("AgentRegistered"));

        let event = DomainEvent::new("TaskCreated", "task-1", serde_json::json!({}));
        projection.handle(&event).await.unwrap();

        assert_eq!(projection.get_position().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_metrics_projection() {
        let projection = MetricsProjection::new();

        let event1 = DomainEvent::new("TaskCreated", "task-1", serde_json::json!({}));
        projection.handle(&event1).await.unwrap();

        let event2 = DomainEvent::new("TaskCompleted", "task-1", serde_json::json!({}));
        projection.handle(&event2).await.unwrap();

        let metrics = projection.get_metrics().await;
        assert_eq!(metrics.total_tasks_created, 1);
        assert_eq!(metrics.total_tasks_completed, 1);
    }
}
