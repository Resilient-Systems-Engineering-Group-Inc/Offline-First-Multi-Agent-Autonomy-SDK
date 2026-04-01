//! Core knowledge graph implementation.

use crate::error::{KnowledgeGraphError, Result};
use crate::types::{
    Entity, EntityId, EntityQuery, Path, Relationship, RelationshipId, RelationshipQuery,
    TraversalDirection,
};
use dashmap::DashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

/// Knowledge graph for storing entities and relationships.
pub struct KnowledgeGraph {
    /// Directed graph structure.
    graph: DiGraph<EntityId, RelationshipId>,
    /// Entity storage.
    entities: DashMap<EntityId, Entity>,
    /// Relationship storage.
    relationships: DashMap<RelationshipId, Relationship>,
    /// Mapping from entity ID to node index.
    entity_to_node: DashMap<EntityId, NodeIndex>,
    /// Mapping from node index to entity ID.
    node_to_entity: DashMap<NodeIndex, EntityId>,
    /// Mapping from relationship ID to edge index.
    relationship_to_edge: DashMap<RelationshipId, petgraph::graph::EdgeIndex>,
}

impl KnowledgeGraph {
    /// Create a new empty knowledge graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            entities: DashMap::new(),
            relationships: DashMap::new(),
            entity_to_node: DashMap::new(),
            node_to_entity: DashMap::new(),
            relationship_to_edge: DashMap::new(),
        }
    }

    /// Add an entity to the graph.
    pub fn add_entity(&self, entity: Entity) -> Result<()> {
        let entity_id = entity.id.clone();
        
        if self.entities.contains_key(&entity_id) {
            return Err(KnowledgeGraphError::EntityAlreadyExists(entity_id));
        }
        
        // Add to graph
        let node_index = self.graph.add_node(entity_id.clone());
        
        // Store mappings
        self.entity_to_node.insert(entity_id.clone(), node_index);
        self.node_to_entity.insert(node_index, entity_id.clone());
        
        // Store entity
        self.entities.insert(entity_id, entity);
        
        debug!("Added entity to knowledge graph");
        Ok(())
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, entity_id: &str) -> Option<Entity> {
        self.entities.get(entity_id).map(|e| e.clone())
    }

    /// Update an entity.
    pub fn update_entity(&self, entity: Entity) -> Result<()> {
        let entity_id = entity.id.clone();
        
        if !self.entities.contains_key(&entity_id) {
            return Err(KnowledgeGraphError::EntityNotFound(entity_id));
        }
        
        self.entities.insert(entity_id, entity);
        Ok(())
    }

    /// Remove an entity from the graph.
    pub fn remove_entity(&self, entity_id: &str) -> Result<()> {
        // Get node index
        let node_index = match self.entity_to_node.get(entity_id) {
            Some(node) => *node.value(),
            None => return Err(KnowledgeGraphError::EntityNotFound(entity_id.to_string())),
        };
        
        // Remove all relationships connected to this entity
        let relationships_to_remove: Vec<RelationshipId> = self
            .relationships
            .iter()
            .filter(|r| r.source == entity_id || r.target == entity_id)
            .map(|r| r.id.clone())
            .collect();
        
        for rel_id in relationships_to_remove {
            self.remove_relationship(&rel_id)?;
        }
        
        // Remove from graph
        self.graph.remove_node(node_index);
        
        // Remove mappings
        self.entity_to_node.remove(entity_id);
        self.node_to_entity.remove(&node_index);
        
        // Remove entity
        self.entities.remove(entity_id);
        
        debug!("Removed entity {} from knowledge graph", entity_id);
        Ok(())
    }

    /// Add a relationship between entities.
    pub fn add_relationship(&self, relationship: Relationship) -> Result<()> {
        let relationship_id = relationship.id.clone();
        
        if self.relationships.contains_key(&relationship_id) {
            return Err(KnowledgeGraphError::RelationshipAlreadyExists(
                relationship_id,
            ));
        }
        
        // Check if source and target entities exist
        let source_node = match self.entity_to_node.get(&relationship.source) {
            Some(node) => *node.value(),
            None => {
                return Err(KnowledgeGraphError::EntityNotFound(relationship.source));
            }
        };
        
        let target_node = match self.entity_to_node.get(&relationship.target) {
            Some(node) => *node.value(),
            None => {
                return Err(KnowledgeGraphError::EntityNotFound(relationship.target));
            }
        };
        
        // Add edge to graph
        let edge_index = self.graph.add_edge(source_node, target_node, relationship_id.clone());
        
        // Store mapping
        self.relationship_to_edge
            .insert(relationship_id.clone(), edge_index);
        
        // Store relationship
        self.relationships.insert(relationship_id, relationship);
        
        debug!("Added relationship to knowledge graph");
        Ok(())
    }

    /// Get a relationship by ID.
    pub fn get_relationship(&self, relationship_id: &str) -> Option<Relationship> {
        self.relationships.get(relationship_id).map(|r| r.clone())
    }

    /// Update a relationship.
    pub fn update_relationship(&self, relationship: Relationship) -> Result<()> {
        let relationship_id = relationship.id.clone();
        
        if !self.relationships.contains_key(&relationship_id) {
            return Err(KnowledgeGraphError::RelationshipNotFound(
                relationship_id,
            ));
        }
        
        self.relationships.insert(relationship_id, relationship);
        Ok(())
    }

    /// Remove a relationship from the graph.
    pub fn remove_relationship(&self, relationship_id: &str) -> Result<()> {
        // Get edge index
        let edge_index = match self.relationship_to_edge.get(relationship_id) {
            Some(edge) => *edge.value(),
            None => {
                return Err(KnowledgeGraphError::RelationshipNotFound(
                    relationship_id.to_string(),
                ));
            }
        };
        
        // Remove from graph
        self.graph.remove_edge(edge_index);
        
        // Remove mappings
        self.relationship_to_edge.remove(relationship_id);
        
        // Remove relationship
        self.relationships.remove(relationship_id);
        
        debug!("Removed relationship {} from knowledge graph", relationship_id);
        Ok(())
    }

    /// Query entities based on criteria.
    pub fn query_entities(&self, query: &EntityQuery) -> Vec<Entity> {
        let mut results = Vec::new();
        
        for entry in self.entities.iter() {
            let entity = entry.value();
            
            // Filter by entity type
            if let Some(ref entity_type) = query.entity_type {
                if &entity.entity_type != entity_type {
                    continue;
                }
            }
            
            // Filter by properties
            let mut matches_all = true;
            for (key, value) in &query.property_filters {
                match entity.get_property(key) {
                    Some(entity_value) => {
                        if entity_value != value {
                            matches_all = false;
                            break;
                        }
                    }
                    None => {
                        matches_all = false;
                        break;
                    }
                }
            }
            
            if matches_all {
                results.push(entity.clone());
            }
        }
        
        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        
        results
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// Query relationships based on criteria.
    pub fn query_relationships(&self, query: &RelationshipQuery) -> Vec<Relationship> {
        let mut results = Vec::new();
        
        for entry in self.relationships.iter() {
            let relationship = entry.value();
            
            // Filter by source
            if let Some(ref source) = query.source {
                if &relationship.source != source {
                    continue;
                }
            }
            
            // Filter by target
            if let Some(ref target) = query.target {
                if &relationship.target != target {
                    continue;
                }
            }
            
            // Filter by relationship type
            if let Some(ref relationship_type) = query.relationship_type {
                if &relationship.relationship_type != relationship_type {
                    continue;
                }
            }
            
            // Filter by properties
            let mut matches_all = true;
            for (key, value) in &query.property_filters {
                match relationship.get_property(key) {
                    Some(rel_value) => {
                        if rel_value != value {
                            matches_all = false;
                            break;
                        }
                    }
                    None => {
                        matches_all = false;
                        break;
                    }
                }
            }
            
            if matches_all {
                results.push(relationship.clone());
            }
        }
        
        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        
        results
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// Find relationships for an entity.
    pub fn get_entity_relationships(
        &self,
        entity_id: &str,
        direction: TraversalDirection,
    ) -> Result<Vec<Relationship>> {
        // Check if entity exists
        if !self.entities.contains_key(entity_id) {
            return Err(KnowledgeGraphError::EntityNotFound(entity_id.to_string()));
        }
        
        let mut relationships = Vec::new();
        
        match direction {
            TraversalDirection::Outgoing => {
                for entry in self.relationships.iter() {
                    let relationship = entry.value();
                    if relationship.source == entity_id {
                        relationships.push(relationship.clone());
                    }
                }
            }
            TraversalDirection::Incoming => {
                for entry in self.relationships.iter() {
                    let relationship = entry.value();
                    if relationship.target == entity_id {
                        relationships.push(relationship.clone());
                    }
                }
            }
            TraversalDirection::Both => {
                for entry in self.relationships.iter() {
                    let relationship = entry.value();
                    if relationship.source == entity_id || relationship.target == entity_id {
                        relationships.push(relationship.clone());
                    }
                }
            }
        }
        
        Ok(relationships)
    }

    /// Find a path between two entities.
    pub fn find_path(&self, start_id: &str, end_id: &str, max_depth: usize) -> Result<Option<Path>> {
        // Check if entities exist
        if !self.entities.contains_key(start_id) {
            return Err(KnowledgeGraphError::EntityNotFound(start_id.to_string()));
        }
        if !self.entities.contains_key(end_id) {
            return Err(KnowledgeGraphError::EntityNotFound(end_id.to_string()));
        }
        
        // Get node indices
        let start_node = match self.entity_to_node.get(start_id) {
            Some(node) => *node.value(),
            None => return Ok(None),
        };
        
        let end_node = match self.entity_to_node.get(end_id) {
            Some(node) => *node.value(),
            None => return Ok(None),
        };
        
        // Use DFS to find a path
        let mut dfs = Dfs::new(&self.graph, start_node);
        let mut visited = HashSet::new();
        let mut parent_map = HashMap::new();
        let mut depth = 0;
        
        while let Some(node) = dfs.next(&self.graph) {
            if depth > max_depth {
                break;
            }
            
            if node == end_node {
                // Reconstruct path
                let mut path_nodes = Vec::new();
                let mut current = node;
                
                while let Some(&parent) = parent_map.get(&current) {
                    path_nodes.push(current);
                    current = parent;
                    if current == start_node {
                        break;
                    }
                }
                path_nodes.push(start_node);
                path_nodes.reverse();
                
                // Convert to entities and relationships
                let entities: Vec<Entity> = path_nodes
                    .iter()
                    .filter_map(|&node_idx| {
                        self.node_to_entity
                            .get(&node_idx)
                            .and_then(|entity_id| self.get_entity(entity_id.value()))
                    })
                    .collect();
                
                let relationships: Vec<Relationship> = path_nodes
                    .windows(2)
                    .filter_map(|window| {
                        let source_node = window[0];
                        let target_node = window[1];
                        
                        // Find relationship between these nodes
                        self.graph
                            .edges_connecting(source_node, target_node)
                            .next()
                            .and_then(|edge| {
                                let rel_id = edge.weight();
                                self.get_relationship(rel_id)
                            })
                    })
                    .collect();
                
                if entities.len() >= 2 && relationships.len() >= 1 {
                    return Ok(Some(Path::new(entities, relationships)));
                } else {
                    return Ok(None);
                }
            }
            
            // Track parent for path reconstruction
            for neighbor in self.graph.neighbors(node) {
                if !visited.contains(&neighbor) {
                    parent_map.insert(neighbor, node);
                    visited.insert(neighbor);
                }
            }
            
            depth += 1;
        }
        
        Ok(None)
    }

    /// Get all entities in the graph.
    pub fn get_all_entities(&self) -> Vec<Entity> {
        self.entities.iter().map(|e| e.clone()).collect()
    }

    /// Get all relationships in the graph.
    pub fn get_all_relationships(&self) -> Vec<Relationship> {
        self.relationships.iter().map(|r| r.clone()).collect()
    }

    /// Get the number of entities in the graph.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get the number of relationships in the graph.
    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    /// Clear the entire graph.
    pub fn clear(&self) {
        self.graph.clear();
        self.entities.clear();
        self.relationships.clear();
        self.entity_to_node.clear();
        self.node_to_entity.clear();
        self.relationship_to_edge.clear();
        
        info!("Cleared knowledge graph");
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entity;
    use serde_json::json;

    #[test]
    fn test_knowledge_graph_creation() {
        let graph = KnowledgeGraph::new();
        assert_eq!(graph.entity_count(), 0);
        assert_eq!(graph.relationship_count(), 0);
    }

    #[test]
    fn test_add_and_get_entity() {
        let graph = KnowledgeGraph::new();
        let entity = Entity::new("Person");
        let entity_id = entity.id.clone();
        
        graph.add_entity(entity.clone()).unwrap();
        assert_eq!(graph.entity_count(), 1);
        
        let retrieved = graph.get_entity(&entity_id).unwrap();
        assert_eq!(retrieved.id, entity_id);
        assert_eq!(retrieved.entity_type, "Person");
    }

    #[test]
    fn test_add_relationship() {
        let graph = KnowledgeGraph::new();
        
        // Add entities
        let entity1 = Entity::new("Person");
        let entity2 = Entity::new("Organization");
        
        graph.add_entity(entity1.clone()).unwrap();
        graph.add_entity(entity2.clone()).unwrap();
        
        // Add relationship
        let relationship = Relationship::new(&entity1.id, &entity2.id, "works_for");
        graph.add_relationship(relationship.clone()).unwrap();
        
        assert_eq!(graph.relationship_count(), 1);
        
        // Query relationships
        let relationships = graph
            .get_entity_relationships(&entity1.id, TraversalDirection::Outgoing)
            .unwrap();
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].relationship_type, "works_for");
    }

    #[test]
    fn test_query_entities() {
        let graph = KnowledgeGraph::new();
        
        // Add entities
        let mut entity1 = Entity::new("Person");
        entity1.set_property("name", json!("Alice"));
        entity1.set_property("age", json!(30));
        
        let mut entity2 = Entity::new("Person");
        entity2.set_property("name", json!("Bob"));
        entity2.set_property("age", json!(25));
        
        graph.add_entity(entity1).unwrap();
        graph.add_entity(entity2).unwrap();
        
        // Query by property
        let query = EntityQuery::new()
            .with_entity_type("Person")
            .with_property_filter("age", json!(30));
        
        let results = graph.query_entities(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].get_property("name"),
            Some(&json!("Alice"))
        );
    }

    #[test]
    fn test_remove_entity() {
        let graph = KnowledgeGraph::new();
        let entity = Entity::new("Person");
        let entity_id = entity.id.clone();
        
        graph.add_entity(entity).unwrap();
        assert_eq!(graph.entity_count(), 1);
        
        graph.remove_entity(&entity_id).unwrap();
        assert_eq!(graph.entity_count(), 0);
        assert!(graph.get_entity(&entity_id).is_none());
    }
}