//! Ontology support for knowledge graphs.
//!
//! This module provides ontology-based knowledge representation with
//! classes, properties, hierarchies, and basic reasoning capabilities.
//! It supports OWL‑like semantics for building rich knowledge models.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during ontology operations.
#[derive(Error, Debug)]
pub enum OntologyError {
    #[error("Class not found: {0}")]
    ClassNotFound(String),
    #[error("Property not found: {0}")]
    PropertyNotFound(String),
    #[error("Invalid hierarchy: {0}")]
    InvalidHierarchy(String),
    #[error("Inconsistent ontology: {0}")]
    Inconsistent(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for ontology operations.
pub type Result<T> = std::result::Result<T, OntologyError>;

/// An ontology class (concept/type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    /// Unique identifier for the class.
    pub id: String,
    /// Human‑readable label.
    pub label: String,
    /// Description of the class.
    pub description: Option<String>,
    /// Parent classes (superclasses).
    pub parents: HashSet<String>,
    /// Equivalent classes.
    pub equivalents: HashSet<String>,
    /// Disjoint classes.
    pub disjoint_with: HashSet<String>,
    /// Properties that can be used with this class.
    pub properties: HashSet<String>,
}

impl Class {
    /// Creates a new class.
    pub fn new(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            description: None,
            parents: HashSet::new(),
            equivalents: HashSet::new(),
            disjoint_with: HashSet::new(),
            properties: HashSet::new(),
        }
    }

    /// Adds a parent class.
    pub fn add_parent(&mut self, parent_id: &str) {
        self.parents.insert(parent_id.to_string());
    }

    /// Adds an equivalent class.
    pub fn add_equivalent(&mut self, equivalent_id: &str) {
        self.equivalents.insert(equivalent_id.to_string());
    }

    /// Adds a disjoint class.
    pub fn add_disjoint(&mut self, disjoint_id: &str) {
        self.disjoint_with.insert(disjoint_id.to_string());
    }

    /// Adds a property that can be used with this class.
    pub fn add_property(&mut self, property_id: &str) {
        self.properties.insert(property_id.to_string());
    }

    /// Checks if this class is a subclass of another class (direct or indirect).
    pub fn is_subclass_of(&self, class_id: &str, ontology: &Ontology) -> bool {
        if self.parents.contains(class_id) {
            return true;
        }
        // Check indirect inheritance via parent classes
        for parent_id in &self.parents {
            if let Some(parent) = ontology.get_class(parent_id) {
                if parent.is_subclass_of(class_id, ontology) {
                    return true;
                }
            }
        }
        false
    }
}

/// An ontology property (relation/attribute).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    /// Unique identifier for the property.
    pub id: String,
    /// Human‑readable label.
    pub label: String,
    /// Description of the property.
    pub description: Option<String>,
    /// Property type (ObjectProperty, DatatypeProperty, AnnotationProperty).
    pub property_type: PropertyType,
    /// Domain (classes that can have this property).
    pub domain: HashSet<String>,
    /// Range (classes or datatypes that can be values).
    pub range: HashSet<String>,
    /// Parent properties (superproperties).
    pub parents: HashSet<String>,
    /// Inverse property.
    pub inverse: Option<String>,
    /// Whether the property is transitive.
    pub transitive: bool,
    /// Whether the property is symmetric.
    pub symmetric: bool,
    /// Whether the property is functional (has at most one value).
    pub functional: bool,
}

/// Type of ontology property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyType {
    /// Relates individuals to individuals.
    ObjectProperty,
    /// Relates individuals to data values.
    DatatypeProperty,
    /// Used for annotations (metadata).
    AnnotationProperty,
}

impl Property {
    /// Creates a new property.
    pub fn new(id: &str, label: &str, property_type: PropertyType) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            description: None,
            property_type,
            domain: HashSet::new(),
            range: HashSet::new(),
            parents: HashSet::new(),
            inverse: None,
            transitive: false,
            symmetric: false,
            functional: false,
        }
    }

    /// Adds a domain class.
    pub fn add_domain(&mut self, class_id: &str) {
        self.domain.insert(class_id.to_string());
    }

    /// Adds a range class or datatype.
    pub fn add_range(&mut self, range: &str) {
        self.range.insert(range.to_string());
    }

    /// Sets the inverse property.
    pub fn set_inverse(&mut self, inverse_id: &str) {
        self.inverse = Some(inverse_id.to_string());
    }
}

/// A complete ontology with classes and properties.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ontology {
    /// Classes in the ontology.
    classes: HashMap<String, Class>,
    /// Properties in the ontology.
    properties: HashMap<String, Property>,
    /// Namespace prefix for the ontology.
    pub namespace: String,
    /// Version of the ontology.
    pub version: String,
}

impl Ontology {
    /// Creates a new empty ontology.
    pub fn new(namespace: &str) -> Self {
        Self {
            classes: HashMap::new(),
            properties: HashMap::new(),
            namespace: namespace.to_string(),
            version: "1.0.0".to_string(),
        }
    }

    /// Adds a class to the ontology.
    pub fn add_class(&mut self, class: Class) -> Result<()> {
        let id = class.id.clone();
        self.classes.insert(id.clone(), class);
        Ok(())
    }

    /// Gets a class by ID.
    pub fn get_class(&self, class_id: &str) -> Option<&Class> {
        self.classes.get(class_id)
    }

    /// Gets a mutable reference to a class.
    pub fn get_class_mut(&mut self, class_id: &str) -> Option<&mut Class> {
        self.classes.get_mut(class_id)
    }

    /// Adds a property to the ontology.
    pub fn add_property(&mut self, property: Property) -> Result<()> {
        let id = property.id.clone();
        self.properties.insert(id.clone(), property);
        Ok(())
    }

    /// Gets a property by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&Property> {
        self.properties.get(property_id)
    }

    /// Gets all subclasses of a given class (including indirect).
    pub fn get_subclasses(&self, class_id: &str) -> HashSet<String> {
        let mut subclasses = HashSet::new();
        for (id, class) in &self.classes {
            if class.is_subclass_of(class_id, self) {
                subclasses.insert(id.clone());
            }
        }
        subclasses
    }

    /// Gets all instances of a class (from a knowledge graph).
    /// This requires integration with the knowledge graph.
    pub fn get_instances(&self, _class_id: &str) -> HashSet<String> {
        // In a real implementation, this would query the knowledge graph
        // For now, return empty set
        HashSet::new()
    }

    /// Validates the ontology for consistency.
    pub fn validate(&self) -> Result<()> {
        // Check for cycles in class hierarchy
        for (class_id, class) in &self.classes {
            if self.has_cycle(class_id, &mut HashSet::new()) {
                return Err(OntologyError::InvalidHierarchy(format!(
                    "Cycle detected in hierarchy of class {}",
                    class_id
                )));
            }
        }

        // Check property domains and ranges reference existing classes
        for (prop_id, property) in &self.properties {
            for domain_class in &property.domain {
                if !self.classes.contains_key(domain_class) {
                    return Err(OntologyError::Inconsistent(format!(
                        "Property {} has domain class {} which does not exist",
                        prop_id, domain_class
                    )));
                }
            }
            for range_class in &property.range {
                if property.property_type == PropertyType::ObjectProperty
                    && !self.classes.contains_key(range_class)
                {
                    // For object properties, range should be a class
                    return Err(OntologyError::Inconsistent(format!(
                        "Object property {} has range class {} which does not exist",
                        prop_id, range_class
                    )));
                }
            }
        }

        Ok(())
    }

    /// Checks for cycles in the class hierarchy (DFS).
    fn has_cycle(&self, class_id: &str, visited: &mut HashSet<String>) -> bool {
        if visited.contains(class_id) {
            return true;
        }
        visited.insert(class_id.to_string());

        if let Some(class) = self.classes.get(class_id) {
            for parent_id in &class.parents {
                if self.has_cycle(parent_id, visited) {
                    return true;
                }
            }
        }

        visited.remove(class_id);
        false
    }

    /// Infers the type of an entity based on its properties.
    pub fn infer_type(&self, entity_properties: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let mut possible_classes = Vec::new();

        for (class_id, class) in &self.classes {
            // Check if entity has all required properties for this class
            let mut matches = true;
            for property_id in &class.properties {
                if !entity_properties.contains_key(property_id) {
                    matches = false;
                    break;
                }
            }
            if matches {
                possible_classes.push(class_id.clone());
            }
        }

        possible_classes
    }

    /// Exports the ontology to RDF/Turtle format (simplified).
    pub fn to_turtle(&self) -> String {
        let mut turtle = String::new();
        turtle.push_str(&format!("@prefix : <{}#> .\n", self.namespace));
        turtle.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        turtle.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
        turtle.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n");

        // Export classes
        for (class_id, class) in &self.classes {
            turtle.push_str(&format!(":{} a owl:Class ;\n", class_id));
            turtle.push_str(&format!("  rdfs:label \"{}\" ;\n", class.label));
            if let Some(desc) = &class.description {
                turtle.push_str(&format!("  rdfs:comment \"{}\" ;\n", desc));
            }
            for parent_id in &class.parents {
                turtle.push_str(&format!("  rdfs:subClassOf :{} ;\n", parent_id));
            }
            turtle.push_str("  .\n\n");
        }

        // Export properties
        for (prop_id, property) in &self.properties {
            let prop_type = match property.property_type {
                PropertyType::ObjectProperty => "owl:ObjectProperty",
                PropertyType::DatatypeProperty => "owl:DatatypeProperty",
                PropertyType::AnnotationProperty => "owl:AnnotationProperty",
            };
            turtle.push_str(&format!(":{} a {} ;\n", prop_id, prop_type));
            turtle.push_str(&format!("  rdfs:label \"{}\" ;\n", property.label));
            if let Some(desc) = &property.description {
                turtle.push_str(&format!("  rdfs:comment \"{}\" ;\n", desc));
            }
            for domain_id in &property.domain {
                turtle.push_str(&format!("  rdfs:domain :{} ;\n", domain_id));
            }
            for range_id in &property.range {
                turtle.push_str(&format!("  rdfs:range :{} ;\n", range_id));
            }
            if let Some(inverse_id) = &property.inverse {
                turtle.push_str(&format!("  owl:inverseOf :{} ;\n", inverse_id));
            }
            if property.transitive {
                turtle.push_str("  a owl:TransitiveProperty ;\n");
            }
            if property.symmetric {
                turtle.push_str("  a owl:SymmetricProperty ;\n");
            }
            if property.functional {
                turtle.push_str("  a owl:FunctionalProperty ;\n");
            }
            turtle.push_str("  .\n\n");
        }

        turtle
    }
}

/// Predefined ontologies for common domains.
pub mod predefined {
    use super::*;

    /// Creates a simple agent ontology.
    pub fn agent_ontology() -> Ontology {
        let mut ontology = Ontology::new("http://example.org/agent#");

        // Classes
        let mut agent_class = Class::new("Agent", "Autonomous agent");
        agent_class.description = Some("An autonomous entity that can perceive and act".to_string());
        ontology.add_class(agent_class).unwrap();

        let mut task_class = Class::new("Task", "Task to be performed");
        task_class.description = Some("A unit of work that can be assigned to an agent".to_string());
        ontology.add_class(task_class).unwrap();

        let mut resource_class = Class::new("Resource", "Computational resource");
        resource_class.description = Some("CPU, memory, network, or other resource".to_string());
        ontology.add_class(resource_class).unwrap();

        // Properties
        let mut has_task = Property::new("hasTask", "has task", PropertyType::ObjectProperty);
        has_task.add_domain("Agent");
        has_task.add_range("Task");
        ontology.add_property(has_task).unwrap();

        let mut requires_resource = Property::new("requiresResource", "requires resource", PropertyType::ObjectProperty);
        requires_resource.add_domain("Task");
        requires_resource.add_range("Resource");
        ontology.add_property(requires_resource).unwrap();

        let mut has_capability = Property::new("hasCapability", "has capability", PropertyType::ObjectProperty);
        has_capability.add_domain("Agent");
        has_capability.add_range("Resource");
        ontology.add_property(has_capability).unwrap();

        ontology
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_hierarchy() {
        let mut ontology = Ontology::new("http://test.org/");
        
        let mut animal = Class::new("Animal", "Animal");
        ontology.add_class(animal).unwrap();
        
        let mut mammal = Class::new("Mammal", "Mammal");
        mammal.add_parent("Animal");
        ontology.add_class(mammal).unwrap();
        
        let mut dog = Class::new("Dog", "Dog");
        dog.add_parent("Mammal");
        ontology.add_class(dog).unwrap();
        
        assert!(ontology.get_class("Dog").unwrap().is_subclass_of("Animal", &ontology));
        assert!(ontology.get_class("Dog").unwrap().is_subclass_of("Mammal", &ontology));
        assert!(!ontology.get_class("Animal").unwrap().is_subclass_of("Dog", &ontology));
    }

    #[test]
    fn test_ontology_validation() {
        let mut ontology = Ontology::new("http://test.org/");
        
        let mut class_a = Class::new("ClassA", "Class A");
        let mut class_b = Class::new("ClassB", "Class B");
        
        // Create a cycle
        class_a.add_parent("ClassB");
        class_b.add_parent("ClassA");
        
        ontology.add_class(class_a).unwrap();
        ontology.add_class(class_b).unwrap();
        
        assert!(ontology.validate().is_err());
    }

    #[test]
    fn test_property_domain_range() {
        let mut ontology = Ontology::new("http://test.org/");
        
        let person = Class::new("Person", "Person");
        ontology.add_class(person).unwrap();
        
        let mut has_name = Property::new("hasName", "has name", PropertyType::DatatypeProperty);
        has_name.add_domain("Person");
        has_name.add_range("xsd:string");
        ontology.add_property(has_name).unwrap();
        
        assert!(ontology.validate().is_ok());
    }
}