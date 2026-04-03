//! Incident data models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for an incident.
pub type IncidentId = Uuid;

/// Severity of an incident.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncidentSeverity {
    /// Informational – no immediate action required.
    Info,
    /// Warning – potential issue.
    Warning,
    /// Error – service degraded.
    Error,
    /// Critical – service outage.
    Critical,
}

/// Status of an incident.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentStatus {
    /// Newly detected, not yet acknowledged.
    New,
    /// Acknowledged by an operator.
    Acknowledged,
    /// Being investigated.
    Investigating,
    /// Being resolved.
    Resolving,
    /// Resolved.
    Resolved,
    /// Closed.
    Closed,
    /// False positive.
    FalsePositive,
}

/// Source of an incident.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentSource {
    /// System monitoring (e.g., CPU, memory).
    SystemMonitoring,
    /// Application logs.
    ApplicationLogs,
    /// Network monitoring.
    NetworkMonitoring,
    /// User report.
    UserReport,
    /// Automated test.
    AutomatedTest,
    /// External service.
    ExternalService,
    /// Custom source.
    Custom(String),
}

/// An incident record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    /// Unique incident ID.
    pub id: IncidentId,
    /// Short title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Severity.
    pub severity: IncidentSeverity,
    /// Current status.
    pub status: IncidentStatus,
    /// Source of the incident.
    pub source: IncidentSource,
    /// Component or service affected.
    pub component: Option<String>,
    /// Agent ID where incident originated.
    pub agent_id: Option<crate::common::types::AgentId>,
    /// Timestamp of detection.
    pub detected_at: DateTime<Utc>,
    /// Timestamp of last update.
    pub updated_at: DateTime<Utc>,
    /// Timestamp of resolution (if resolved).
    pub resolved_at: Option<DateTime<Utc>>,
    /// Assigned owner (agent ID or user).
    pub assigned_to: Option<String>,
    /// Related resource IDs (e.g., task IDs, workflow IDs).
    pub related_resources: Vec<String>,
    /// Additional metadata (JSON).
    pub metadata: serde_json::Value,
}

impl Incident {
    /// Create a new incident.
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        severity: IncidentSeverity,
        source: IncidentSource,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: description.into(),
            severity,
            status: IncidentStatus::New,
            source,
            component: None,
            agent_id: None,
            detected_at: now,
            updated_at: now,
            resolved_at: None,
            assigned_to: None,
            related_resources: Vec::new(),
            metadata: serde_json::json!({}),
        }
    }

    /// Update the status and set updated_at.
    pub fn update_status(&mut self, status: IncidentStatus) {
        self.status = status;
        self.updated_at = Utc::now();
        if status == IncidentStatus::Resolved || status == IncidentStatus::Closed {
            self.resolved_at = Some(Utc::now());
        }
    }

    /// Assign the incident to an owner.
    pub fn assign_to(&mut self, owner: impl Into<String>) {
        self.assigned_to = Some(owner.into());
        self.updated_at = Utc::now();
    }

    /// Add a related resource.
    pub fn add_related_resource(&mut self, resource: impl Into<String>) {
        self.related_resources.push(resource.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_creation() {
        let incident = Incident::new(
            "High CPU",
            "CPU usage above 90%",
            IncidentSeverity::Warning,
            IncidentSource::SystemMonitoring,
        );
        assert_eq!(incident.severity, IncidentSeverity::Warning);
        assert_eq!(incident.status, IncidentStatus::New);
    }

    #[test]
    fn test_update_status() {
        let mut incident = Incident::new("test", "test", IncidentSeverity::Info, IncidentSource::Custom("test".to_string()));
        incident.update_status(IncidentStatus::Acknowledged);
        assert_eq!(incident.status, IncidentStatus::Acknowledged);
        assert!(incident.updated_at > incident.detected_at);
    }
}