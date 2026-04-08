//! Knowledge graph for representing and querying relationships between entities.
//!
//! This crate provides a graph-based knowledge representation system for agents
//! to store and query relationships between entities, with ontology support
//! for semantic reasoning.
//!
//! # Features
//! - Entity and relationship management
//! - Property-based querying
//! - Graph traversal and path finding
//! - SPARQL-like query support (optional feature)
//! - Ontology support with classes, properties, and reasoning
//! - RDF/Turtle export for interoperability
//!
//! # Example
//! ```
//! use knowledge_graph::{KnowledgeGraph, Entity, Relationship};
//!
//! let graph = KnowledgeGraph::new();
//!
//! // Add entities
//! let mut person = Entity::new("Person");
//! person.set_property("name", serde_json::json!("Alice"));
//! person.set_property("age", serde_json::json!(30));
//!
//! let mut company = Entity::new("Organization");
//! company.set_property("name", serde_json::json!("Acme Corp"));
//!
//! graph.add_entity(person.clone()).unwrap();
//! graph.add_entity(company.clone()).unwrap();
//!
//! // Add relationship
//! let relationship = Relationship::new(&person.id, &company.id, "works_for");
//! graph.add_relationship(relationship).unwrap();
//!
//! // Query entities
//! let query = knowledge_graph::EntityQuery::new()
//!     .with_entity_type("Person")
//!     .with_property_filter("age", serde_json::json!(30));
//!
//! let results = graph.query_entities(&query);
//! assert_eq!(results.len(), 1);
//! ```
//!

pub mod error;
pub mod graph;
pub mod types;
pub mod ontology;

// Re-export commonly used types
pub use error::{KnowledgeGraphError, Result};
pub use graph::KnowledgeGraph;
pub use types::{
    Entity, EntityId, EntityQuery, Path, Relationship, RelationshipId, RelationshipQuery,
    TraversalDirection,
};
pub use ontology::{Ontology, Class, Property, PropertyType, OntologyError};

/// Current version of the knowledge graph crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the knowledge graph system.
pub fn init() {
    // Any initialization logic would go here
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_basic_graph_operations() {
        let graph = KnowledgeGraph::new();
        
        // Add entity
        let entity = Entity::new("Person");
        let entity_id = entity.id.clone();
        
        graph.add_entity(entity).unwrap();
        assert_eq!(graph.entity_count(), 1);
        
        // Get entity
        let retrieved = graph.get_entity(&entity_id).unwrap();
        assert_eq!(retrieved.id, entity_id);
        
        // Remove entity
        graph.remove_entity(&entity_id).unwrap();
        assert_eq!(graph.entity_count(), 0);
    }

    #[test]
    fn test_relationship_operations() {
        let graph = KnowledgeGraph::new();
        
        // Add entities
        let entity1 = Entity::new("Person");
        let entity2 = Entity::new("Organization");
        
        graph.add_entity(entity1.clone()).unwrap();
        graph.add_entity(entity2.clone()).unwrap();
        
        // Add relationship
        let relationship = Relationship::new(&entity1.id, &entity2.id, "works_for");
        let relationship_id = relationship.id.clone();
        
        graph.add_relationship(relationship).unwrap();
        assert_eq!(graph.relationship_count(), 1);
        
        // Get relationship
        let retrieved = graph.get_relationship(&relationship_id).unwrap();
        assert_eq!(retrieved.id, relationship_id);
        
        // Remove relationship
        graph.remove_relationship(&relationship_id).unwrap();
        assert_eq!(graph.relationship_count(), 0);
    }

    #[test]
    fn test_entity_query() {
        let graph = KnowledgeGraph::new();
        
        // Add entities with properties
        let mut entity1 = Entity::new("Person");
        entity1.set_property("name", json!("Alice"));
        entity1.set_property("age", json!(30));
        
        let mut entity2 = Entity::new("Person");
        entity2.set_property("name", json!("Bob"));
        entity2.set_property("age", json!(25));
        
        graph.add_entity(entity1).unwrap();
        graph.add_entity(entity2).unwrap();
        
        // Query for Alice
        let query = EntityQuery::new()
            .with_entity_type("Person")
            .with_property_filter("name", json!("Alice"));
        
        let results = graph.query_entities(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].get_property("name"),
            Some(&json!("Alice"))
        );
        
        // Query for people over 28
        let query = EntityQuery::new()
            .with_entity_type("Person")
            .with_property_filter("age", json!(30));
        
        let results = graph.query_entities(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].get_property("age"),
            Some(&json!(30))
        );
    }
}