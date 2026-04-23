//! Domain events for event sourcing.

use serde::{Deserialize, Serialize};

/// Domain event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    pub event_type: String,
    pub aggregate_id: String,
    pub data: serde_json::Value,
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent {
    pub fn new(event_type: &str, aggregate_id: &str, data: serde_json::Value) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.to_string(),
            aggregate_id: aggregate_id.to_string(),
            data,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    // Task events
    TaskCreated,
    TaskAssigned,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskCancelled,

    // Agent events
    AgentRegistered,
    AgentConnected,
    AgentDisconnected,
    AgentStatusChanged,
    AgentCapabilityAdded,

    // Workflow events
    WorkflowStarted,
    WorkflowStepCompleted,
    WorkflowCompleted,
    WorkflowFailed,

    // System events
    SystemStarted,
    SystemStopped,
    ConfigChanged,
    AlertTriggered,
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "TaskCreated" => EventType::TaskCreated,
            "TaskAssigned" => EventType::TaskAssigned,
            "TaskStarted" => EventType::TaskStarted,
            "TaskCompleted" => EventType::TaskCompleted,
            "TaskFailed" => EventType::TaskFailed,
            "TaskCancelled" => EventType::TaskCancelled,
            "AgentRegistered" => EventType::AgentRegistered,
            "AgentConnected" => EventType::AgentConnected,
            "AgentDisconnected" => EventType::AgentDisconnected,
            "AgentStatusChanged" => EventType::AgentStatusChanged,
            "AgentCapabilityAdded" => EventType::AgentCapabilityAdded,
            "WorkflowStarted" => EventType::WorkflowStarted,
            "WorkflowStepCompleted" => EventType::WorkflowStepCompleted,
            "WorkflowCompleted" => EventType::WorkflowCompleted,
            "WorkflowFailed" => EventType::WorkflowFailed,
            "SystemStarted" => EventType::SystemStarted,
            "SystemStopped" => EventType::SystemStopped,
            "ConfigChanged" => EventType::ConfigChanged,
            "AlertTriggered" => EventType::AlertTriggered,
            _ => EventType::TaskCreated, // Default
        }
    }
}

impl From<EventType> for String {
    fn from(event_type: EventType) -> Self {
        format!("{:?}", event_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event() {
        let event = DomainEvent::new(
            "TaskCreated",
            "task-1",
            serde_json::json!({"description": "Test task"}),
        );

        assert_eq!(event.event_type, "TaskCreated");
        assert_eq!(event.aggregate_id, "task-1");
        assert!(!event.event_id.is_empty());
    }

    #[test]
    fn test_event_type_conversion() {
        let event_type: EventType = "TaskCreated".into();
        assert!(matches!(event_type, EventType::TaskCreated));

        let event_type: EventType = "Unknown".into();
        assert!(matches!(event_type, EventType::TaskCreated)); // Default
    }
}
