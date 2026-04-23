//! Audit event definitions.

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub category: EventCategory,
    pub severity: EventSeverity,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub action: String,
    pub result: AuditResult,
    pub details: serde_json::Value,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub previous_hash: Option<String>,
    pub signature: Option<String>,
}

impl AuditEvent {
    pub fn new(
        event_type: &str,
        entity_type: &str,
        entity_id: &str,
        action: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_type: event_type.to_string(),
            category: EventCategory::from(event_type),
            severity: EventSeverity::Info,
            entity_type: Some(entity_type.to_string()),
            entity_id: Some(entity_id.to_string()),
            user_id: None,
            session_id: None,
            action: action.to_string(),
            result: AuditResult::Success,
            details: serde_json::json!({}),
            metadata: serde_json::json!({}),
            ip_address: None,
            user_agent: None,
            previous_hash: None,
            signature: None,
        }
    }

    pub fn with_user_id(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_ip_address(mut self, ip_address: &str) -> Self {
        self.ip_address = Some(ip_address.to_string());
        self
    }

    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }

    pub fn with_previous_hash(mut self, hash: &str) -> Self {
        self.previous_hash = Some(hash.to_string());
        self
    }

    /// Compute hash for tamper-proof chain.
    pub fn compute_hash(&self, previous_hash: &str) -> String {
        let mut hasher = Sha256::new();
        
        hasher.update(previous_hash.as_bytes());
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.event_type.as_bytes());
        hasher.update(self.entity_type.as_ref().unwrap_or(&String::new()).as_bytes());
        hasher.update(self.entity_id.as_ref().unwrap_or(&String::new()).as_bytes());
        hasher.update(self.action.as_bytes());
        
        let hash = hasher.finalize();
        hex::encode(hash)
    }

    /// Sign event with private key (for non-repudiation).
    pub fn sign(&mut self, private_key: &[u8]) -> Result<(), String> {
        // Simplified signing - would use actual crypto in production
        let hash = self.compute_hash(self.previous_hash.as_ref().unwrap_or(&String::new()));
        self.signature = Some(format!("sig_{}", hash));
        Ok(())
    }

    /// Verify event signature.
    pub fn verify(&self, public_key: &[u8]) -> bool {
        // Simplified verification
        if let Some(sig) = &self.signature {
            let expected = format!("sig_{}", self.compute_hash(self.previous_hash.as_ref().unwrap_or(&String::new())));
            sig == &expected
        } else {
            false
        }
    }
}

/// Event category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventCategory {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    System,
    Security,
    Configuration,
    Workflow,
    Custom,
}

impl From<&str> for EventCategory {
    fn from(event_type: &str) -> Self {
        if event_type.contains("auth") || event_type.contains("login") || event_type.contains("logout") {
            Self::Authentication
        } else if event_type.contains("permission") || event_type.contains("role") {
            Self::Authorization
        } else if event_type.contains("read") || event_type.contains("get") || event_type.contains("list") {
            Self::DataAccess
        } else if event_type.contains("create") || event_type.contains("update") || event_type.contains("delete") {
            Self::DataModification
        } else if event_type.contains("system") || event_type.contains("startup") || event_type.contains("shutdown") {
            Self::System
        } else if event_type.contains("security") || event_type.contains("violation") || event_type.contains("threat") {
            Self::Security
        } else if event_type.contains("config") || event_type.contains("setting") {
            Self::Configuration
        } else if event_type.contains("workflow") || event_type.contains("task") {
            Self::Workflow
        } else {
            Self::Custom
        }
    }
}

/// Event severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    Debug,
}

/// Audit result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub success: bool,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub duration_ms: Option<f64>,
}

impl AuditResult {
    pub fn success() -> Self {
        Self {
            success: true,
            status_code: Some(200),
            error_message: None,
            duration_ms: None,
        }
    }

    pub fn failure(status_code: i32, error_message: &str) -> Self {
        Self {
            success: false,
            status_code: Some(status_code),
            error_message: Some(error_message.to_string()),
            duration_ms: None,
        }
    }

    pub fn with_duration(mut self, duration_ms: f64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// Common audit event types.
pub mod event_types {
    // Authentication
    pub const USER_LOGIN: &str = "auth.user.login";
    pub const USER_LOGOUT: &str = "auth.user.logout";
    pub const USER_LOGIN_FAILED: &str = "auth.user.login_failed";
    pub const PASSWORD_CHANGED: &str = "auth.password.changed";
    pub const PASSWORD_RESET: &str = "auth.password.reset";
    pub const MFA_ENABLED: &str = "auth.mfa.enabled";
    pub const MFA_DISABLED: &str = "auth.mfa.disabled";

    // Authorization
    pub const PERMISSION_GRANTED: &str = "auth.permission.granted";
    pub const PERMISSION_REVOKED: &str = "auth.permission.revoked";
    pub const ROLE_ASSIGNED: &str = "auth.role.assigned";
    pub const ROLE_REMOVED: &str = "auth.role.removed";

    // Data Access
    pub const DATA_READ: &str = "data.read";
    pub const DATA_EXPORT: &str = "data.export";
    pub const DATA_SEARCH: &str = "data.search";

    // Data Modification
    pub const DATA_CREATED: &str = "data.created";
    pub const DATA_UPDATED: &str = "data.updated";
    pub const DATA_DELETED: &str = "data.deleted";
    pub const DATA_BULK_IMPORT: &str = "data.bulk_import";

    // System
    pub const SYSTEM_STARTUP: &str = "system.startup";
    pub const SYSTEM_SHUTDOWN: &str = "system.shutdown";
    pub const SYSTEM_CONFIG_CHANGED: &str = "system.config.changed";

    // Security
    pub const SECURITY_VIOLATION: &str = "security.violation";
    pub const SECURITY_THREAT_DETECTED: &str = "security.threat.detected";
    pub const ACCESS_DENIED: &str = "security.access_denied";
    pub const RATE_LIMIT_EXCEEDED: &str = "security.rate_limit.exceeded";

    // Workflow
    pub const WORKFLOW_STARTED: &str = "workflow.started";
    pub const WORKFLOW_COMPLETED: &str = "workflow.completed";
    pub const WORKFLOW_FAILED: &str = "workflow.failed";
    pub const TASK_ASSIGNED: &str = "workflow.task.assigned";
    pub const TASK_COMPLETED: &str = "workflow.task.completed";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event() {
        let event = AuditEvent::new(
            "task.created",
            "task",
            "task-1",
            "create",
        )
        .with_user_id("user-123")
        .with_details(serde_json::json!({
            "description": "Test task"
        }));

        assert_eq!(event.event_type, "task.created");
        assert_eq!(event.user_id, Some("user-123".to_string()));
        assert_eq!(event.category, EventCategory::Workflow);
    }

    #[test]
    fn test_hash_chain() {
        let event1 = AuditEvent::new("test", "entity", "1", "create");
        let hash1 = event1.compute_hash("");
        
        let mut event2 = AuditEvent::new("test", "entity", "2", "update");
        event2.previous_hash = Some(hash1.clone());
        let hash2 = event2.compute_hash(&hash1);

        assert_ne!(hash1, hash2);
        assert!(!hash1.is_empty());
        assert!(!hash2.is_empty());
    }

    #[test]
    fn test_event_categories() {
        assert_eq!(EventCategory::from("auth.user.login"), EventCategory::Authentication);
        assert_eq!(EventCategory::from("data.created"), EventCategory::DataModification);
        assert_eq!(EventCategory::from("security.violation"), EventCategory::Security);
        assert_eq!(EventCategory::from("workflow.task.assigned"), EventCategory::Workflow);
    }
}
