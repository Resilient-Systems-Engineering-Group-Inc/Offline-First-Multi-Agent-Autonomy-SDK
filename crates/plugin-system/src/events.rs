//! Plugin event system.

use serde::{Deserialize, Serialize};

/// Plugin event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

impl Event {
    pub fn new(event_type: &str, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            data,
        }
    }

    pub fn task_created(task_id: &str, description: &str) -> Self {
        Self::new("task_created", serde_json::json!({
            "task_id": task_id,
            "description": description
        }))
    }

    pub fn task_completed(task_id: &str, result: &serde_json::Value) -> Self {
        Self::new("task_completed", serde_json::json!({
            "task_id": task_id,
            "result": result
        }))
    }

    pub fn agent_connected(agent_id: &str) -> Self {
        Self::new("agent_connected", serde_json::json!({
            "agent_id": agent_id
        }))
    }

    pub fn agent_disconnected(agent_id: &str) -> Self {
        Self::new("agent_disconnected", serde_json::json!({
            "agent_id": agent_id
        }))
    }

    pub fn workflow_started(workflow_id: &str) -> Self {
        Self::new("workflow_started", serde_json::json!({
            "workflow_id": workflow_id
        }))
    }

    pub fn workflow_completed(workflow_id: &str, output: &serde_json::Value) -> Self {
        Self::new("workflow_completed", serde_json::json!({
            "workflow_id": workflow_id,
            "output": output
        }))
    }
}

/// Event type enumeration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    TaskCreated,
    TaskCompleted,
    TaskFailed,
    AgentConnected,
    AgentDisconnected,
    WorkflowStarted,
    WorkflowCompleted,
    MetricsUpdated,
    ConfigChanged,
    Custom(String),
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "task_created" => EventType::TaskCreated,
            "task_completed" => EventType::TaskCompleted,
            "task_failed" => EventType::TaskFailed,
            "agent_connected" => EventType::AgentConnected,
            "agent_disconnected" => EventType::AgentDisconnected,
            "workflow_started" => EventType::WorkflowStarted,
            "workflow_completed" => EventType::WorkflowCompleted,
            "metrics_updated" => EventType::MetricsUpdated,
            "config_changed" => EventType::ConfigChanged,
            other => EventType::Custom(other.to_string()),
        }
    }
}

impl From<EventType> for String {
    fn from(event_type: EventType) -> Self {
        match event_type {
            EventType::TaskCreated => "task_created".to_string(),
            EventType::TaskCompleted => "task_completed".to_string(),
            EventType::TaskFailed => "task_failed".to_string(),
            EventType::AgentConnected => "agent_connected".to_string(),
            EventType::AgentDisconnected => "agent_disconnected".to_string(),
            EventType::WorkflowStarted => "workflow_started".to_string(),
            EventType::WorkflowCompleted => "workflow_completed".to_string(),
            EventType::MetricsUpdated => "metrics_updated".to_string(),
            EventType::ConfigChanged => "config_changed".to_string(),
            EventType::Custom(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::task_created("task-1", "Test task");
        assert_eq!(event.event_type, "task_created");
        assert_eq!(event.data["task_id"], "task-1");
    }

    #[test]
    fn test_event_type_conversion() {
        let event_type: EventType = "task_created".into();
        assert!(matches!(event_type, EventType::TaskCreated));

        let event_type: EventType = "custom_event".into();
        assert!(matches!(event_type, EventType::Custom(_)));
    }
}
