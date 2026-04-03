//! Metadata data models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for a metadata entry.
pub type MetadataId = Uuid;

/// Type of metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetadataType {
    /// Agent metadata.
    Agent,
    /// Task metadata.
    Task,
    /// Workflow metadata.
    Workflow,
    /// Resource metadata.
    Resource,
    /// Sensor metadata.
    Sensor,
    /// Actuator metadata.
    Actuator,
    /// Custom type.
    Custom(String),
}

/// Metadata schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSchema {
    /// Schema ID.
    pub id: Uuid,
    /// Schema name.
    pub name: String,
    /// Schema version.
    pub version: String,
    /// JSON Schema definition.
    pub definition: serde_json::Value,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
}

/// A metadata entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Unique ID.
    pub id: MetadataId,
    /// Type of metadata.
    pub metadata_type: MetadataType,
    /// ID of the entity this metadata belongs to.
    pub entity_id: String,
    /// Schema ID (optional).
    pub schema_id: Option<Uuid>,
    /// Metadata content (JSON).
    pub content: serde_json::Value,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
    /// Version number (monotonically increasing).
    pub version: u64,
    /// Agent ID that created this metadata.
    pub author_agent_id: Option<crate::common::types::AgentId>,
}

impl Metadata {
    /// Create a new metadata entry.
    pub fn new(
        metadata_type: MetadataType,
        entity_id: impl Into<String>,
        content: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            metadata_type,
            entity_id: entity_id.into(),
            schema_id: None,
            content,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            version: 1,
            author_agent_id: None,
        }
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
        self.updated_at = Utc::now();
    }

    /// Update content and increment version.
    pub fn update_content(&mut self, content: serde_json::Value) {
        self.content = content;
        self.version += 1;
        self.updated_at = Utc::now();
    }

    /// Validate against a schema (placeholder).
    pub fn validate(&self, _schema: &MetadataSchema) -> bool {
        // In a real implementation, you would use a JSON Schema validator.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let metadata = Metadata::new(
            MetadataType::Agent,
            "agent-123",
            serde_json::json!({ "name": "test" }),
        );
        assert_eq!(metadata.metadata_type, MetadataType::Agent);
        assert_eq!(metadata.entity_id, "agent-123");
        assert_eq!(metadata.version, 1);
    }
}