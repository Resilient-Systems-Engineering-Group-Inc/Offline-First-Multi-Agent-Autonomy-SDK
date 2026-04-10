//! Digital twin model representation.
//!
//! This module defines the core data structures for representing digital twins,
//! including entities, properties, relationships, and state.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Error;

/// Unique identifier for a digital twin entity.
pub type EntityId = Uuid;

/// Type of a digital twin entity (e.g., robot, sensor, room, machine).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// Physical robot (mobile or stationary).
    Robot,
    /// Sensor device (temperature, pressure, camera, etc.).
    Sensor,
    /// Actuator device (motor, valve, light, etc.).
    Actuator,
    /// Room or area in a building.
    Room,
    /// Industrial machine or equipment.
    Machine,
    /// Human operator or user.
    Human,
    /// Virtual entity (software agent, service).
    Virtual,
    /// Custom entity type.
    Custom(String),
}

/// Physical or logical property of an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    /// Property name.
    pub name: String,
    /// Property value (JSON).
    pub value: serde_json::Value,
    /// Data type of the property.
    pub data_type: String,
    /// Unit of measurement (optional).
    pub unit: Option<String>,
    /// Timestamp when the property was last updated.
    pub timestamp: DateTime<Utc>,
    /// Quality/confidence of the measurement (0.0 to 1.0).
    pub confidence: Option<f64>,
    /// Metadata (source, calibration info, etc.).
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Relationship identifier.
    pub id: Uuid,
    /// Source entity ID.
    pub source: EntityId,
    /// Target entity ID.
    pub target: EntityId,
    /// Relationship type (e.g., "contains", "connected_to", "controls").
    pub relationship_type: String,
    /// Relationship properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Timestamp when the relationship was created.
    pub created_at: DateTime<Utc>,
}

/// State of an entity (position, orientation, operational status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    /// Entity identifier.
    pub entity_id: EntityId,
    /// Timestamp of this state.
    pub timestamp: DateTime<Utc>,
    /// Position in 3D space (x, y, z) in meters.
    pub position: Option<(f64, f64, f64)>,
    /// Orientation as quaternion (x, y, z, w).
    pub orientation: Option<(f64, f64, f64, f64)>,
    /// Velocity vector (vx, vy, vz) in m/s.
    pub velocity: Option<(f64, f64, f64)>,
    /// Angular velocity vector (wx, wy, wz) in rad/s.
    pub angular_velocity: Option<(f64, f64, f64)>,
    /// Operational status.
    pub status: OperationalStatus,
    /// Battery level (0.0 to 1.0) if applicable.
    pub battery_level: Option<f64>,
    /// Temperature in Celsius if applicable.
    pub temperature: Option<f64>,
    /// Custom state fields.
    pub custom_state: HashMap<String, serde_json::Value>,
}

/// Operational status of an entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationalStatus {
    /// Fully operational.
    Operational,
    /// Degraded performance.
    Degraded,
    /// Maintenance required.
    Maintenance,
    /// Failed/unresponsive.
    Failed,
    /// Powered off.
    Off,
    /// Unknown status.
    Unknown,
}

/// Digital twin entity.
#[derive(Debug, Clone)]
pub struct Entity {
    /// Unique identifier.
    pub id: EntityId,
    /// Human-readable name.
    pub name: String,
    /// Entity type.
    pub entity_type: EntityType,
    /// Description.
    pub description: Option<String>,
    /// Properties (key-value pairs).
    pub properties: HashMap<String, Property>,
    /// Current state.
    pub state: Option<EntityState>,
    /// Parent entity ID (if part of a hierarchy).
    pub parent_id: Option<EntityId>,
    /// Child entity IDs.
    pub children: Vec<EntityId>,
    /// Timestamp when the entity was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the entity was last updated.
    pub updated_at: DateTime<Utc>,
    /// Metadata (manufacturer, model, version, etc.).
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Entity {
    /// Create a new entity.
    pub fn new(
        name: String,
        entity_type: EntityType,
        description: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            entity_type,
            description,
            properties: HashMap::new(),
            state: None,
            parent_id: None,
            children: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add or update a property.
    pub fn set_property(
        &mut self,
        name: String,
        value: serde_json::Value,
        data_type: String,
        unit: Option<String>,
        confidence: Option<f64>,
    ) {
        let property = Property {
            name: name.clone(),
            value,
            data_type,
            unit,
            timestamp: Utc::now(),
            confidence,
            metadata: HashMap::new(),
        };
        self.properties.insert(name, property);
        self.updated_at = Utc::now();
    }

    /// Get a property value.
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties.get(name)
    }

    /// Update the entity state.
    pub fn update_state(&mut self, state: EntityState) {
        self.state = Some(state);
        self.updated_at = Utc::now();
    }

    /// Add a child entity.
    pub fn add_child(&mut self, child_id: EntityId) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a child entity.
    pub fn remove_child(&mut self, child_id: &EntityId) -> bool {
        let index = self.children.iter().position(|id| id == child_id);
        if let Some(idx) = index {
            self.children.remove(idx);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }
}

/// Digital twin model representing a collection of entities and relationships.
#[derive(Debug, Clone)]
pub struct DigitalTwinModel {
    /// Model identifier.
    pub id: Uuid,
    /// Model name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// All entities in the model.
    pub entities: HashMap<EntityId, Entity>,
    /// All relationships between entities.
    pub relationships: HashMap<Uuid, Relationship>,
    /// Root entity IDs (entities without parents).
    pub roots: Vec<EntityId>,
    /// Timestamp when the model was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the model was last updated.
    pub updated_at: DateTime<Utc>,
}

impl DigitalTwinModel {
    /// Create a new empty digital twin model.
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            entities: HashMap::new(),
            relationships: HashMap::new(),
            roots: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an entity to the model.
    pub fn add_entity(&mut self, entity: Entity) -> Result<(), Error> {
        if self.entities.contains_key(&entity.id) {
            return Err(Error::Model(format!(
                "Entity with ID {} already exists",
                entity.id
            )));
        }

        self.entities.insert(entity.id, entity);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, entity_id: &EntityId) -> Option<&Entity> {
        self.entities.get(entity_id)
    }

    /// Get a mutable reference to an entity.
    pub fn get_entity_mut(&mut self, entity_id: &EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(entity_id)
    }

    /// Remove an entity and all its relationships.
    pub fn remove_entity(&mut self, entity_id: &EntityId) -> Result<(), Error> {
        // Remove entity
        let entity = self.entities.remove(entity_id)
            .ok_or_else(|| Error::Model(format!("Entity {} not found", entity_id)))?;

        // Remove relationships where this entity is source or target
        let rel_ids: Vec<Uuid> = self.relationships
            .iter()
            .filter(|(_, rel)| rel.source == *entity_id || rel.target == *entity_id)
            .map(|(id, _)| *id)
            .collect();

        for rel_id in rel_ids {
            self.relationships.remove(&rel_id);
        }

        // Remove from roots if present
        if let Some(pos) = self.roots.iter().position(|id| id == entity_id) {
            self.roots.remove(pos);
        }

        // Update parent references in other entities
        for other_entity in self.entities.values_mut() {
            other_entity.remove_child(entity_id);
            if other_entity.parent_id == Some(*entity_id) {
                other_entity.parent_id = None;
            }
        }

        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a relationship between two entities.
    pub fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        relationship_type: String,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<Uuid, Error> {
        // Verify entities exist
        if !self.entities.contains_key(&source) {
            return Err(Error::Model(format!("Source entity {} not found", source)));
        }
        if !self.entities.contains_key(&target) {
            return Err(Error::Model(format!("Target entity {} not found", target)));
        }

        let relationship = Relationship {
            id: Uuid::new_v4(),
            source,
            target,
            relationship_type,
            properties,
            created_at: Utc::now(),
        };

        let id = relationship.id;
        self.relationships.insert(id, relationship);
        self.updated_at = Utc::now();
        Ok(id)
    }

    /// Get relationships for an entity.
    pub fn get_relationships(&self, entity_id: &EntityId) -> Vec<&Relationship> {
        self.relationships
            .values()
            .filter(|rel| rel.source == *entity_id || rel.target == *entity_id)
            .collect()
    }

    /// Find entities by type.
    pub fn find_entities_by_type(&self, entity_type: &EntityType) -> Vec<&Entity> {
        self.entities
            .values()
            .filter(|entity| &entity.entity_type == entity_type)
            .collect()
    }

    /// Get the hierarchical path of an entity (from root to entity).
    pub fn get_entity_path(&self, entity_id: &EntityId) -> Option<Vec<EntityId>> {
        let mut path = Vec::new();
        let mut current = Some(*entity_id);

        while let Some(id) = current {
            path.insert(0, id);
            
            // Find parent
            current = self.entities.get(&id).and_then(|e| e.parent_id);
            
            // Check for cycles
            if path.len() > self.entities.len() {
                return None; // Cycle detected
            }
        }

        Some(path)
    }

    /// Export the model to JSON.
    pub fn to_json(&self) -> Result<serde_json::Value, Error> {
        serde_json::to_value(self).map_err(|e| Error::Model(e.to_string()))
    }

    /// Import a model from JSON.
    pub fn from_json(value: serde_json::Value) -> Result<Self, Error> {
        serde_json::from_value(value).map_err(|e| Error::Model(e.to_string()))
    }
}

/// Main digital twin manager.
pub struct DigitalTwin {
    /// The underlying model.
    model: DigitalTwinModel,
    /// Simulation time (can be real-time or accelerated).
    simulation_time: DateTime<Utc>,
    /// Time scaling factor (1.0 = real-time).
    time_scale: f64,
    /// Whether the simulation is running.
    is_running: bool,
}

impl DigitalTwin {
    /// Create a new digital twin with an empty model.
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            model: DigitalTwinModel::new(name, description),
            simulation_time: Utc::now(),
            time_scale: 1.0,
            is_running: false,
        }
    }

    /// Get the underlying model.
    pub fn model(&self) -> &DigitalTwinModel {
        &self.model
    }

    /// Get a mutable reference to the model.
    pub fn model_mut(&mut self) -> &mut DigitalTwinModel {
        &mut self.model
    }

    /// Start the simulation.
    pub fn start(&mut self) {
        self.is_running = true;
        self.simulation_time = Utc::now();
    }

    /// Stop the simulation.
    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Step the simulation forward by a delta time.
    pub fn step(&mut self, delta: Duration) -> Result<(), Error> {
        if !self.is_running {
            return Ok(());
        }

        // Update simulation time
        let scaled_delta = Duration::from_secs_f64(
            delta.as_secs_f64() * self.time_scale
        );
        // In a real implementation, we would update entity states based on physics
        self.simulation_time += chrono::Duration::from_std(scaled_delta)
            .map_err(|e| Error::Physics(e.to_string()))?;

        Ok(())
    }

    /// Get current simulation time.
    pub fn simulation_time(&self) -> DateTime<Utc> {
        self.simulation_time
    }

    /// Set time scale (e.g., 2.0 for 2x speed, 0.5 for half speed).
    pub fn set_time_scale(&mut self, scale: f64) {
        self.time_scale = scale.max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(
            "Test Robot".to_string(),
            EntityType::Robot,
            Some("A test robot".to_string()),
        );

        assert_eq!(entity.name, "Test Robot");
        assert!(matches!(entity.entity_type, EntityType::Robot));
        assert!(entity.properties.is_empty());
        assert!(entity.state.is_none());
    }

    #[test]
    fn test_entity_properties() {
        let mut entity = Entity::new("Sensor".to_string(), EntityType::Sensor, None);
        
        entity.set_property(
            "temperature".to_string(),
            json!(25.5),
            "float".to_string(),
            Some("°C".to_string()),
            Some(0.95),
        );

        let prop = entity.get_property("temperature").unwrap();
        assert_eq!(prop.value, json!(25.5));
        assert_eq!(prop.unit.as_deref(), Some("°C"));
        assert_eq!(prop.confidence, Some(0.95));
    }

    #[test]
    fn test_digital_twin_model() {
        let mut model = DigitalTwinModel::new(
            "Factory".to_string(),
            Some("A factory digital twin".to_string()),
        );

        let robot = Entity::new("Robot1".to_string(), EntityType::Robot, None);
        let sensor = Entity::new("TempSensor".to_string(), EntityType::Sensor, None);

        model.add_entity(robot).unwrap();
        model.add_entity(sensor).unwrap();

        assert_eq!(model.entities.len(), 2);
    }

    #[test]
    fn test_relationships() {
        let mut model = DigitalTwinModel::new("Test".to_string(), None);
        
        let robot = Entity::new("Robot".to_string(), EntityType::Robot, None);
        let sensor = Entity::new("Sensor".to_string(), EntityType::Sensor, None);
        
        let robot_id = robot.id;
        let sensor_id = sensor.id;
        
        model.add_entity(robot).unwrap();
        model.add_entity(sensor).unwrap();
        
        let rel_id = model.add_relationship(
            robot_id,
            sensor_id,
            "has_sensor".to_string(),
            HashMap::new(),
        ).unwrap();
        
        let relationships = model.get_relationships(&robot_id);
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].id, rel_id);
    }
}