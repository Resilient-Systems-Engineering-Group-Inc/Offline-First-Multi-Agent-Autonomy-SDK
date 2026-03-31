//! Audit event definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Severity of an audit event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Severity {
    /// Informational event.
    Info,
    /// Warning event.
    Warning,
    /// Error event.
    Error,
    /// Security‑related event.
    Security,
}

/// Type of audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// Agent lifecycle (start, stop, join, leave).
    AgentLifecycle,
    /// Task assignment, completion, failure.
    TaskLifecycle,
    /// Configuration change.
    ConfigurationChange,
    /// Security event (authentication, authorization).
    Security,
    /// Resource usage exceeded threshold.
    ResourceAlert,
    /// Network connection established/lost.
    Network,
    /// Custom user‑defined event.
    Custom(String),
}

/// An audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID.
    pub id: Uuid,
    /// Timestamp (UTC).
    pub timestamp: DateTime<Utc>,
    /// Event type.
    pub event_type: EventType,
    /// Severity.
    pub severity: Severity,
    /// Source agent ID (if any).
    pub source_agent: Option<String>,
    /// Target agent ID (if any).
    pub target_agent: Option<String>,
    /// Description.
    pub description: String,
    /// Additional structured data.
    pub payload: serde_json::Value,
}

impl AuditEvent {
    /// Create a new audit event.
    pub fn new(
        event_type: EventType,
        severity: Severity,
        source_agent: Option<String>,
        target_agent: Option<String>,
        description: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            severity,
            source_agent,
            target_agent,
            description,
            payload,
        }
    }

    /// Create an informational agent lifecycle event.
    pub fn agent_lifecycle(
        agent_id: String,
        action: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self::new(
            EventType::AgentLifecycle,
            Severity::Info,
            Some(agent_id),
            None,
            format!("Agent {}", action),
            payload,
        )
    }

    /// Create a warning resource alert.
    pub fn resource_alert(
        agent_id: String,
        resource: &str,
        usage: f64,
        threshold: f64,
    ) -> Self {
        Self::new(
            EventType::ResourceAlert,
            Severity::Warning,
            Some(agent_id),
            None,
            format!("Resource {} usage {:.1}% exceeds threshold {:.1}%", resource, usage, threshold),
            serde_json::json!({ "resource": resource, "usage": usage, "threshold": threshold }),
        )
    }
}