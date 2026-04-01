//! Knowledge graph data types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique entity ID.
pub type EntityId = String;

/// Unique relationship ID.
pub type RelationshipId = String;

/// Entity in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity ID.
    pub id: EntityId,
    /// Entity type (e.g., "Person", "Organization", "Task").
    pub entity_type: String,
    /// Entity properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Timestamp when the entity was created.
    pub created_at: std::time::SystemTime,
    /// Timestamp when the entity was last updated.
    pub updated_at: std::time::SystemTime,
}

impl Entity {
    /// Create a new entity.
    pub fn new(entity_type: &str) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            entity_type: entity_type.to_string(),
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new entity with a specific ID.
    pub fn with_id(id: &str, entity_type: &str) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id: id.to_string(),
            entity_type: entity_type.to_string(),
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set a property on the entity.
    pub fn set_property(&mut self, key: &str, value: serde_json::Value) {
        self.properties.insert(key.to_string(), value);
        self.updated_at = std::time::SystemTime::now();
    }

    /// Get a property from the entity.
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    /// Remove a property from the entity.
    pub fn remove_property(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.properties.remove(key);
        if result.is_some() {
            self.updated_at = std::time::SystemTime::now();
        }
        result
    }

    /// Check if the entity has a property.
    pub fn has_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }
}

/// Relationship between entities in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Relationship ID.
    pub id: RelationshipId,
    /// Source entity ID.
    pub source: EntityId,
    /// Target entity ID.
    pub target: EntityId,
    /// Relationship type (e.g., "works_for", "located_in", "depends_on").
    pub relationship_type: String,
    /// Relationship properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Timestamp when the relationship was created.
    pub created_at: std::time::SystemTime,
    /// Timestamp when the relationship was last updated.
    pub updated_at: std::time::SystemTime,
}

impl Relationship {
    /// Create a new relationship.
    pub fn new(source: &str, target: &str, relationship_type: &str) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            relationship_type: relationship_type.to_string(),
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set a property on the relationship.
    pub fn set_property(&mut self, key: &str, value: serde_json::Value) {
        self.properties.insert(key.to_string(), value);
        self.updated_at = std::time::SystemTime::now();
    }

    /// Get a property from the relationship.
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    /// Check if the relationship has a property.
    pub fn has_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }
}

/// Query for searching entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityQuery {
    /// Entity type to filter by (optional).
    pub entity_type: Option<String>,
    /// Property filters.
    pub property_filters: HashMap<String, serde_json::Value>,
    /// Limit the number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

impl EntityQuery {
    /// Create a new entity query.
    pub fn new() -> Self {
        Self {
            entity_type: None,
            property_filters: HashMap::new(),
            limit: None,
            offset: None,
        }
    }

    /// Filter by entity type.
    pub fn with_entity_type(mut self, entity_type: &str) -> Self {
        self.entity_type = Some(entity_type.to_string());
        self
    }

    /// Add a property filter.
    pub fn with_property_filter(mut self, key: &str, value: serde_json::Value) -> Self {
        self.property_filters.insert(key.to_string(), value);
        self
    }

    /// Set a limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set an offset.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl Default for EntityQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Query for searching relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipQuery {
    /// Source entity ID (optional).
    pub source: Option<EntityId>,
    /// Target entity ID (optional).
    pub target: Option<EntityId>,
    /// Relationship type to filter by (optional).
    pub relationship_type: Option<String>,
    /// Property filters.
    pub property_filters: HashMap<String, serde_json::Value>,
    /// Limit the number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

impl RelationshipQuery {
    /// Create a new relationship query.
    pub fn new() -> Self {
        Self {
            source: None,
            target: None,
            relationship_type: None,
            property_filters: HashMap::new(),
            limit: None,
            offset: None,
        }
    }

    /// Filter by source entity.
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    /// Filter by target entity.
    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    /// Filter by relationship type.
    pub fn with_relationship_type(mut self, relationship_type: &str) -> Self {
        self.relationship_type = Some(relationship_type.to_string());
        self
    }

    /// Add a property filter.
    pub fn with_property_filter(mut self, key: &str, value: serde_json::Value) -> Self {
        self.property_filters.insert(key.to_string(), value);
        self
    }
}

impl Default for RelationshipQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph traversal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    /// Traverse from source to target (outgoing relationships).
    Outgoing,
    /// Traverse from target to source (incoming relationships).
    Incoming,
    /// Traverse both directions.
    Both,
}

/// Path in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    /// Entities in the path.
    pub entities: Vec<Entity>,
    /// Relationships in the path.
    pub relationships: Vec<Relationship>,
    /// Total path length (number of relationships).
    pub length: usize,
}

impl Path {
    /// Create a new path.
    pub fn new(entities: Vec<Entity>, relationships: Vec<Relationship>) -> Self {
        let length = relationships.len();
        Self {
            entities,
            relationships,
            length,
        }
    }

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get the start entity of the path.
    pub fn start(&self) -> Option<&Entity> {
        self.entities.first()
    }

    /// Get the end entity of the path.
    pub fn end(&self) -> Option<&Entity> {
        self.entities.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new("Person");
        assert!(!entity.id.is_empty());
        assert_eq!(entity.entity_type, "Person");
        assert!(entity.properties.is_empty());
    }

    #[test]
    fn test_entity_properties() {
        let mut entity = Entity::new("Person");
        entity.set_property("name", json!("John Doe"));
        entity.set_property("age", json!(30));
        
        assert_eq!(entity.get_property("name"), Some(&json!("John Doe")));
        assert_eq!(entity.get_property("age"), Some(&json!(30)));
        assert!(entity.has_property("name"));
        assert!(!entity.has_property("address"));
        
        let removed = entity.remove_property("age");
        assert_eq!(removed, Some(json!(30)));
        assert!(!entity.has_property("age"));
    }

    #[test]
    fn test_relationship_creation() {
        let relationship = Relationship::new("entity-1", "entity-2", "works_for");
        assert!(!relationship.id.is_empty());
        assert_eq!(relationship.source, "entity-1");
        assert_eq!(relationship.target, "entity-2");
        assert_eq!(relationship.relationship_type, "works_for");
    }

    #[test]
    fn test_entity_query() {
        let query = EntityQuery::new()
            .with_entity_type("Person")
            .with_property_filter("age", json!(30))
            .with_limit(10);
        
        assert_eq!(query.entity_type, Some("Person".to_string()));
        assert_eq!(query.property_filters.get("age"), Some(&json!(30)));
        assert_eq!(query.limit, Some(10));
    }
}