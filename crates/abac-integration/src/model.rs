//! ABAC data models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Attribute key‑value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name.
    pub key: String,
    /// Attribute value (JSON).
    pub value: serde_json::Value,
}

impl Attribute {
    /// Create a new attribute.
    pub fn new(key: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

/// Subject (user, agent, service) with attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    /// Unique identifier.
    pub id: String,
    /// Subject type (e.g., "agent", "user").
    pub subject_type: String,
    /// Attributes.
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Subject {
    /// Create a new subject.
    pub fn new(id: impl Into<String>, subject_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            subject_type: subject_type.into(),
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute.
    pub fn add_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }

    /// Get an attribute.
    pub fn get_attribute(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }
}

/// Resource with attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Unique identifier.
    pub id: String,
    /// Resource type (e.g., "task", "workflow").
    pub resource_type: String,
    /// Attributes.
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Resource {
    /// Create a new resource.
    pub fn new(id: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            resource_type: resource_type.into(),
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute.
    pub fn add_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }
}

/// Environment context (time, location, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Attributes.
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Environment {
    /// Create a new empty environment.
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute.
    pub fn add_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// ABAC policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule ID.
    pub id: Uuid,
    /// Description.
    pub description: String,
    /// Target expression (optional).
    pub target: Option<serde_json::Value>,
    /// Condition expression (JSON logic).
    pub condition: serde_json::Value,
    /// Effect ("allow" or "deny").
    pub effect: String,
    /// Priority (higher = more important).
    pub priority: i32,
}

impl PolicyRule {
    /// Create a new policy rule.
    pub fn new(
        description: impl Into<String>,
        condition: serde_json::Value,
        effect: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            target: None,
            condition,
            effect: effect.into(),
            priority: 0,
        }
    }
}

/// ABAC policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy ID.
    pub id: Uuid,
    /// Policy name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Rules.
    pub rules: Vec<PolicyRule>,
    /// Created timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Policy {
    /// Create a new policy.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            rules: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subject_creation() {
        let subject = Subject::new("agent1", "agent");
        assert_eq!(subject.id, "agent1");
    }

    #[test]
    fn test_policy_creation() {
        let policy = Policy::new("test", "test policy");
        assert_eq!(policy.name, "test");
    }
}