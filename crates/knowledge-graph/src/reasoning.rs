//! Logical reasoning and rule‑based inference for knowledge graphs.
//!
//! This module provides rule‑based reasoning capabilities over knowledge graphs,
//! including forward‑chaining inference, ontology‑based classification,
//! and SPARQL‑like query support.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::graph::KnowledgeGraph;
use crate::ontology::{Class, Ontology, Property};
use crate::types::{Entity, EntityId, Relationship};

/// Errors that can occur during reasoning.
#[derive(Error, Debug)]
pub enum ReasoningError {
    #[error("Rule evaluation failed: {0}")]
    RuleEvaluation(String),
    #[error("Invalid rule syntax: {0}")]
    InvalidRule(String),
    #[error("Query execution failed: {0}")]
    QueryExecution(String),
    #[error("Ontology inconsistency: {0}")]
    OntologyInconsistency(String),
}

/// Result type for reasoning operations.
pub type Result<T> = std::result::Result<T, ReasoningError>;

/// A rule condition that can be evaluated against the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Entity has a specific type (class).
    EntityType(String),
    /// Entity has a property with a given value.
    PropertyEquals(String, serde_json::Value),
    /// Entity has a property with value greater than a threshold.
    PropertyGreater(String, f64),
    /// Entity has a property with value less than a threshold.
    PropertyLess(String, f64),
    /// Relationship exists between two entities.
    RelationshipExists(EntityId, EntityId, String),
    /// Logical AND of multiple conditions.
    And(Vec<Condition>),
    /// Logical OR of multiple conditions.
    Or(Vec<Condition>),
    /// Logical NOT of a condition.
    Not(Box<Condition>),
}

impl Condition {
    /// Evaluate the condition against an entity in the context of a knowledge graph.
    pub fn evaluate(&self, entity_id: &EntityId, graph: &KnowledgeGraph) -> bool {
        match self {
            Condition::EntityType(expected_type) => {
                graph.get_entity(entity_id)
                    .map(|e| e.entity_type == *expected_type)
                    .unwrap_or(false)
            }
            Condition::PropertyEquals(key, expected_value) => {
                graph.get_entity(entity_id)
                    .and_then(|e| e.properties.get(key))
                    .map(|value| value == expected_value)
                    .unwrap_or(false)
            }
            Condition::PropertyGreater(key, threshold) => {
                graph.get_entity(entity_id)
                    .and_then(|e| e.properties.get(key))
                    .and_then(|v| v.as_f64())
                    .map(|value| value > *threshold)
                    .unwrap_or(false)
            }
            Condition::PropertyLess(key, threshold) => {
                graph.get_entity(entity_id)
                    .and_then(|e| e.properties.get(key))
                    .and_then(|v| v.as_f64())
                    .map(|value| value < *threshold)
                    .unwrap_or(false)
            }
            Condition::RelationshipExists(source, target, rel_type) => {
                graph.get_relationships()
                    .iter()
                    .any(|r| &r.source == source && &r.target == target && r.relationship_type == *rel_type)
            }
            Condition::And(conditions) => {
                conditions.iter().all(|c| c.evaluate(entity_id, graph))
            }
            Condition::Or(conditions) => {
                conditions.iter().any(|c| c.evaluate(entity_id, graph))
            }
            Condition::Not(condition) => {
                !condition.evaluate(entity_id, graph)
            }
        }
    }
}

/// An action to perform when a rule fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Add a new entity to the graph.
    AddEntity(Entity),
    /// Add a relationship between two existing entities.
    AddRelationship(Relationship),
    /// Update an entity's property.
    SetProperty(EntityId, String, serde_json::Value),
    /// Remove an entity.
    RemoveEntity(EntityId),
    /// Remove a relationship.
    RemoveRelationship(EntityId, EntityId, String),
    /// Log a message (for debugging).
    Log(String),
}

/// A rule consisting of a condition and an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule identifier.
    pub id: String,
    /// Human‑readable description.
    pub description: String,
    /// Condition that must be satisfied for the rule to fire.
    pub condition: Condition,
    /// Action to execute when the condition is satisfied.
    pub action: Action,
    /// Priority (higher priority rules fire first).
    pub priority: i32,
    /// Whether the rule is enabled.
    pub enabled: bool,
}

impl Rule {
    /// Create a new rule.
    pub fn new(id: &str, description: &str, condition: Condition, action: Action) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            condition,
            action,
            priority: 0,
            enabled: true,
        }
    }

    /// Set rule priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Rule engine that applies rules to a knowledge graph.
pub struct RuleEngine {
    /// Registered rules.
    rules: Vec<Rule>,
    /// Maximum number of inference cycles (to prevent infinite loops).
    max_cycles: usize,
}

impl RuleEngine {
    /// Create a new rule engine.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            max_cycles: 100,
        }
    }

    /// Add a rule to the engine.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove a rule by ID.
    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.retain(|r| r.id != rule_id);
    }

    /// Run forward‑chaining inference on the graph.
    pub fn infer(&self, graph: &mut KnowledgeGraph) -> Result<Vec<Action>> {
        let mut fired_actions = Vec::new();
        let mut cycle = 0;

        loop {
            if cycle >= self.max_cycles {
                return Err(ReasoningError::RuleEvaluation(
                    format!("Maximum inference cycles ({}) reached", self.max_cycles)
                ));
            }

            let mut any_fired = false;
            for rule in &self.rules {
                if !rule.enabled {
                    continue;
                }

                // Evaluate rule against all entities
                let entity_ids: Vec<EntityId> = graph.get_entities().iter()
                    .map(|e| e.id.clone())
                    .collect();

                for entity_id in &entity_ids {
                    if rule.condition.evaluate(entity_id, graph) {
                        // Execute action
                        let action = rule.action.clone();
                        self.execute_action(&action, graph)?;
                        fired_actions.push(action);
                        any_fired = true;
                        // After firing, break to re‑evaluate rules (since graph changed)
                        break;
                    }
                }

                if any_fired {
                    break;
                }
            }

            if !any_fired {
                break; // No more rules can fire
            }

            cycle += 1;
        }

        Ok(fired_actions)
    }

    /// Execute a single action on the graph.
    fn execute_action(&self, action: &Action, graph: &mut KnowledgeGraph) -> Result<()> {
        match action {
            Action::AddEntity(entity) => {
                graph.add_entity(entity.clone())
                    .map_err(|e| ReasoningError::RuleEvaluation(e.to_string()))?;
            }
            Action::AddRelationship(rel) => {
                graph.add_relationship(rel.clone())
                    .map_err(|e| ReasoningError::RuleEvaluation(e.to_string()))?;
            }
            Action::SetProperty(entity_id, key, value) => {
                if let Some(entity) = graph.get_entity_mut(entity_id) {
                    entity.set_property(key, value.clone());
                }
            }
            Action::RemoveEntity(entity_id) => {
                graph.remove_entity(entity_id)
                    .map_err(|e| ReasoningError::RuleEvaluation(e.to_string()))?;
            }
            Action::RemoveRelationship(source, target, rel_type) => {
                graph.remove_relationship(source, target, rel_type)
                    .map_err(|e| ReasoningError::RuleEvaluation(e.to_string()))?;
            }
            Action::Log(message) => {
                tracing::debug!("Rule log: {}", message);
            }
        }
        Ok(())
    }

    /// Get all rules.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

/// Ontology‑based reasoner that performs classification and consistency checking.
pub struct OntologyReasoner {
    /// The ontology to reason over.
    ontology: Arc<Ontology>,
}

impl OntologyReasoner {
    /// Create a new reasoner with the given ontology.
    pub fn new(ontology: Arc<Ontology>) -> Self {
        Self { ontology }
    }

    /// Classify entities in the graph based on ontology classes.
    pub fn classify_entities(&self, graph: &mut KnowledgeGraph) -> Result<usize> {
        let mut classified = 0;
        let entities: Vec<Entity> = graph.get_entities().to_vec();

        for mut entity in entities {
            let mut inferred_types = HashSet::new();

            // Check each class in the ontology
            for (class_id, class) in self.ontology.classes() {
                if self.entity_matches_class(&entity, class) {
                    inferred_types.insert(class_id.clone());
                }
            }

            // Add inferred types as properties
            if !inferred_types.is_empty() {
                let types_json: Vec<serde_json::Value> = inferred_types.into_iter()
                    .map(|t| serde_json::Value::String(t))
                    .collect();
                entity.set_property("inferred_types", serde_json::Value::Array(types_json));
                graph.update_entity(entity)?;
                classified += 1;
            }
        }

        Ok(classified)
    }

    /// Check if an entity matches a class definition.
    fn entity_matches_class(&self, entity: &Entity, class: &Class) -> bool {
        // Check if entity has required properties
        for property_id in &class.properties {
            if !entity.properties.contains_key(property_id) {
                return false;
            }
        }
        // Additional matching logic could be added (property value ranges, etc.)
        true
    }

    /// Check ontology consistency (no contradictions).
    pub fn check_consistency(&self) -> Result<()> {
        // Verify class hierarchy is acyclic
        for (class_id, class) in self.ontology.classes() {
            for parent_id in &class.parents {
                if let Some(parent) = self.ontology.get_class(parent_id) {
                    if parent.is_subclass_of(class_id, &self.ontology) {
                        return Err(ReasoningError::OntologyInconsistency(
                            format!("Circular inheritance detected: {} <-> {}", class_id, parent_id)
                        ));
                    }
                }
            }
        }

        // Check disjoint classes
        for (class_id, class) in self.ontology.classes() {
            for disjoint_id in &class.disjoint_with {
                if let Some(disjoint_class) = self.ontology.get_class(disjoint_id) {
                    if disjoint_class.is_subclass_of(class_id, &self.ontology) {
                        return Err(ReasoningError::OntologyInconsistency(
                            format!("Disjoint classes {} and {} are in subclass relationship", class_id, disjoint_id)
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Infer transitive relationships based on ontology property chains.
    pub fn infer_transitive_relationships(&self, graph: &mut KnowledgeGraph) -> Result<usize> {
        let mut inferred = 0;
        let relationships: Vec<Relationship> = graph.get_relationships().to_vec();

        // Simple transitive closure for "subClassOf" and "partOf" relationships
        for rel in &relationships {
            if rel.relationship_type == "subClassOf" || rel.relationship_type == "partOf" {
                // Find chains
                let chain = self.find_transitive_chain(&rel.source, &rel.target, &rel.relationship_type, graph);
                for (source, target) in chain {
                    if !graph.has_relationship(&source, &target, &rel.relationship_type) {
                        let new_rel = Relationship::new(&source, &target, &rel.relationship_type);
                        graph.add_relationship(new_rel)?;
                        inferred += 1;
                    }
                }
            }
        }

        Ok(inferred)
    }

    /// Find transitive chain using depth‑first search.
    fn find_transitive_chain(
        &self,
        source: &EntityId,
        target: &EntityId,
        rel_type: &str,
        graph: &KnowledgeGraph,
    ) -> Vec<(EntityId, EntityId)> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![target.clone()];

        while let Some(node) = stack.pop() {
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node.clone());

            // Find relationships where node is target
            for rel in graph.get_relationships() {
                if rel.relationship_type == rel_type && rel.target == node {
                    chain.push((source.clone(), rel.source.clone()));
                    stack.push(rel.source.clone());
                }
            }
        }

        chain
    }
}

/// SPARQL‑like query engine for knowledge graphs.
pub struct QueryEngine {
    /// Optional ontology for semantic query expansion.
    ontology: Option<Arc<Ontology>>,
}

impl QueryEngine {
    /// Create a new query engine.
    pub fn new(ontology: Option<Arc<Ontology>>) -> Self {
        Self { ontology }
    }

    /// Execute a SPARQL‑like query.
    pub fn execute(&self, query: &str, graph: &KnowledgeGraph) -> Result<QueryResult> {
        // Simplified query parsing (for demonstration)
        // In a real implementation, this would parse SPARQL or a similar language.
        if query.trim().starts_with("SELECT") {
            self.execute_select(query, graph)
        } else if query.trim().starts_with("ASK") {
            self.execute_ask(query, graph)
        } else {
            Err(ReasoningError::QueryExecution("Unsupported query type".into()))
        }
    }

    /// Execute a SELECT query.
    fn execute_select(&self, query: &str, graph: &KnowledgeGraph) -> Result<QueryResult> {
        // Parse variables (simplified)
        let variables: Vec<String> = query.split_whitespace()
            .skip_while(|&w| w != "SELECT")
            .take_while(|&w| w != "WHERE")
            .filter(|&w| w != "SELECT" && w != "*")
            .map(|s| s.trim_matches('?').to_string())
            .collect();

        // For demonstration, return all entities
        let mut results = Vec::new();
        for entity in graph.get_entities() {
            let mut row = HashMap::new();
            for var in &variables {
                if var == "id" {
                    row.insert(var.clone(), serde_json::Value::String(entity.id.clone()));
                } else if var == "type" {
                    row.insert(var.clone(), serde_json::Value::String(entity.entity_type.clone()));
                } else if let Some(value) = entity.properties.get(var) {
                    row.insert(var.clone(), value.clone());
                }
            }
            if !row.is_empty() {
                results.push(row);
            }
        }

        Ok(QueryResult::Select { variables, results })
    }

    /// Execute an ASK query (boolean).
    fn execute_ask(&self, query: &str, graph: &KnowledgeGraph) -> Result<QueryResult> {
        // Simplified: check if any entity exists
        let answer = !graph.get_entities().is_empty();
        Ok(QueryResult::Ask { answer })
    }
}

/// Result of a query execution.
#[derive(Debug, Clone)]
pub enum QueryResult {
    /// SELECT query result with variables and rows.
    Select {
        variables: Vec<String>,
        results: Vec<HashMap<String, serde_json::Value>>,
    },
    /// ASK query result (boolean).
    Ask {
        answer: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::KnowledgeGraph;
    use crate::ontology::Ontology;

    #[test]
    fn test_rule_condition_evaluation() {
        let mut graph = KnowledgeGraph::new();
        let mut entity = Entity::new("Person");
        entity.set_property("age", serde_json::json!(25));
        entity.set_property("name", serde_json::json!("Alice"));
        graph.add_entity(entity.clone()).unwrap();

        let condition = Condition::And(vec![
            Condition::EntityType("Person".to_string()),
            Condition::PropertyEquals("age".to_string(), serde_json::json!(25)),
        ]);

        assert!(condition.evaluate(&entity.id, &graph));
    }

    #[test]
    fn test_rule_engine() {
        let mut graph = KnowledgeGraph::new();
        let entity = Entity::new("Person");
        graph.add_entity(entity.clone()).unwrap();

        let rule = Rule::new(
            "test-rule",
            "Add a property to Person entities",
            Condition::EntityType("Person".to_string()),
            Action::SetProperty(entity.id.clone(), "processed".to_string(), serde_json::json!(true)),
        );

        let mut engine = RuleEngine::new();
        engine.add_rule(rule);
        let actions = engine.infer(&mut graph).unwrap();

        assert_eq!(actions.len(), 1);
        assert!(graph.get_entity(&entity.id).unwrap().properties.contains_key("processed"));
    }

    #[test]
    fn test_ontology_reasoner() {
        let ontology = Arc::new(Ontology::new());
        let reasoner = OntologyReasoner::new(ontology);
        let mut graph = KnowledgeGraph::new();

        // Consistency check should pass for empty ontology
        assert!(reasoner.check_consistency().is_ok());

        let classified = reasoner.classify_entities(&mut graph).unwrap();
        assert_eq!(classified, 0);
    }
}