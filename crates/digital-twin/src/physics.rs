//! Physics simulation for digital twins.
//!
//! This module provides basic physics simulation capabilities for digital twin
//! entities, including motion, collisions, and environmental interactions.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

use crate::error::Error;
use crate::model::{Entity, EntityId, EntityState, OperationalStatus};

/// Physics engine configuration.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    /// Gravity vector (m/s²).
    pub gravity: (f64, f64, f64),
    /// Air density (kg/m³).
    pub air_density: f64,
    /// Time step for simulation (seconds).
    pub time_step: f64,
    /// Whether to enable collision detection.
    pub enable_collisions: bool,
    /// Whether to enable friction.
    pub enable_friction: bool,
    /// Maximum simulation speed (time scale).
    pub max_time_scale: f64,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: (0.0, 0.0, -9.81), // Earth gravity
            air_density: 1.225, // Sea level
            time_step: 0.01, // 10 ms
            enable_collisions: true,
            enable_friction: true,
            max_time_scale: 10.0,
        }
    }
}

/// Collision shape for physics simulation.
#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// Sphere with given radius.
    Sphere { radius: f64 },
    /// Axis-aligned bounding box (width, height, depth).
    Box { width: f64, height: f64, depth: f64 },
    /// Cylinder with radius and height.
    Cylinder { radius: f64, height: f64 },
    /// Capsule with radius and height.
    Capsule { radius: f64, height: f64 },
    /// Mesh (convex hull) - vertices in local coordinates.
    Mesh { vertices: Vec<(f64, f64, f64)> },
}

impl CollisionShape {
    /// Compute bounding sphere radius.
    pub fn bounding_radius(&self) -> f64 {
        match self {
            CollisionShape::Sphere { radius } => *radius,
            CollisionShape::Box { width, height, depth } => {
                (width * width + height * height + depth * depth).sqrt() / 2.0
            }
            CollisionShape::Cylinder { radius, height } => {
                (radius * radius + (height / 2.0) * (height / 2.0)).sqrt()
            }
            CollisionShape::Capsule { radius, height } => radius + height / 2.0,
            CollisionShape::Mesh { vertices } => {
                vertices.iter()
                    .map(|(x, y, z)| (x * x + y * y + z * z).sqrt())
                    .fold(0.0, f64::max)
            }
        }
    }

    /// Compute volume.
    pub fn volume(&self) -> f64 {
        match self {
            CollisionShape::Sphere { radius } => (4.0 / 3.0) * PI * radius * radius * radius,
            CollisionShape::Box { width, height, depth } => width * height * depth,
            CollisionShape::Cylinder { radius, height } => PI * radius * radius * height,
            CollisionShape::Capsule { radius, height } => {
                let sphere_volume = (4.0 / 3.0) * PI * radius * radius * radius;
                let cylinder_volume = PI * radius * radius * height;
                sphere_volume + cylinder_volume
            }
            CollisionShape::Mesh { .. } => 0.0, // Complex to compute
        }
    }
}

/// Physical properties of an entity.
#[derive(Debug, Clone)]
pub struct PhysicalProperties {
    /// Mass in kilograms.
    pub mass: f64,
    /// Collision shape.
    pub collision_shape: CollisionShape,
    /// Coefficient of restitution (bounciness, 0.0 to 1.0).
    pub restitution: f64,
    /// Coefficient of friction (0.0 to 1.0).
    pub friction: f64,
    /// Drag coefficient.
    pub drag_coefficient: f64,
    /// Cross-sectional area for drag calculation (m²).
    pub cross_sectional_area: f64,
    /// Whether the entity is static (immovable).
    pub is_static: bool,
}

impl Default for PhysicalProperties {
    fn default() -> Self {
        Self {
            mass: 1.0,
            collision_shape: CollisionShape::Sphere { radius: 0.5 },
            restitution: 0.5,
            friction: 0.3,
            drag_coefficient: 0.47, // Sphere
            cross_sectional_area: 0.785, // π * 0.5²
            is_static: false,
        }
    }
}

/// Collision between two entities.
#[derive(Debug, Clone)]
pub struct Collision {
    /// First entity ID.
    pub entity_a: EntityId,
    /// Second entity ID.
    pub entity_b: EntityId,
    /// Collision point (world coordinates).
    pub point: (f64, f64, f64),
    /// Collision normal (from a to b).
    pub normal: (f64, f64, f64),
    /// Penetration depth.
    pub depth: f64,
    /// Relative velocity at collision point.
    pub relative_velocity: (f64, f64, f64),
}

/// Physics engine for simulating digital twin entities.
pub struct PhysicsEngine {
    config: PhysicsConfig,
    entities: HashMap<EntityId, PhysicalProperties>,
    forces: HashMap<EntityId, Vec<(f64, f64, f64)>>, // Accumulated forces per entity
    collisions: Vec<Collision>,
}

impl PhysicsEngine {
    /// Create a new physics engine with the given configuration.
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            config,
            entities: HashMap::new(),
            forces: HashMap::new(),
            collisions: Vec::new(),
        }
    }

    /// Register an entity with physical properties.
    pub fn register_entity(
        &mut self,
        entity_id: EntityId,
        properties: PhysicalProperties,
    ) -> Result<(), Error> {
        if self.entities.contains_key(&entity_id) {
            return Err(Error::Physics(format!(
                "Entity {} already registered",
                entity_id
            )));
        }

        self.entities.insert(entity_id, properties);
        self.forces.insert(entity_id, Vec::new());
        Ok(())
    }

    /// Unregister an entity.
    pub fn unregister_entity(&mut self, entity_id: &EntityId) {
        self.entities.remove(entity_id);
        self.forces.remove(entity_id);
    }

    /// Apply a force to an entity.
    pub fn apply_force(
        &mut self,
        entity_id: &EntityId,
        force: (f64, f64, f64),
    ) -> Result<(), Error> {
        if let Some(forces) = self.forces.get_mut(entity_id) {
            forces.push(force);
            Ok(())
        } else {
            Err(Error::Physics(format!("Entity {} not found", entity_id)))
        }
    }

    /// Apply an impulse (instantaneous change in momentum) to an entity.
    pub fn apply_impulse(
        &mut self,
        entity_id: &EntityId,
        impulse: (f64, f64, f64),
        entity_state: &mut EntityState,
    ) -> Result<(), Error> {
        let properties = self.entities.get(entity_id)
            .ok_or_else(|| Error::Physics(format!("Entity {} not found", entity_id)))?;

        if properties.is_static {
            return Ok(());
        }

        // Δv = impulse / mass
        let mass = properties.mass;
        let delta_v = (
            impulse.0 / mass,
            impulse.1 / mass,
            impulse.2 / mass,
        );

        // Update velocity
        if let Some(vel) = entity_state.velocity {
            entity_state.velocity = Some((
                vel.0 + delta_v.0,
                vel.1 + delta_v.1,
                vel.2 + delta_v.2,
            ));
        } else {
            entity_state.velocity = Some(delta_v);
        }

        Ok(())
    }

    /// Perform a simulation step.
    pub fn step(
        &mut self,
        entity_states: &mut HashMap<EntityId, EntityState>,
    ) -> Result<Vec<Collision>, Error> {
        self.collisions.clear();

        // Clear accumulated forces and apply gravity
        for (entity_id, properties) in &self.entities {
            if !properties.is_static {
                let mut forces = Vec::new();
                
                // Gravity force: F = m * g
                let gravity_force = (
                    properties.mass * self.config.gravity.0,
                    properties.mass * self.config.gravity.1,
                    properties.mass * self.config.gravity.2,
                );
                forces.push(gravity_force);
                
                self.forces.insert(*entity_id, forces);
            }
        }

        // Update entity states based on forces
        for (entity_id, properties) in &self.entities {
            if properties.is_static {
                continue;
            }

            if let Some(state) = entity_states.get_mut(entity_id) {
                self.update_entity_state(entity_id, properties, state)?;
            }
        }

        // Detect collisions if enabled
        if self.config.enable_collisions {
            self.detect_collisions(entity_states)?;
        }

        // Resolve collisions
        for collision in &self.collisions {
            self.resolve_collision(collision, entity_states)?;
        }

        Ok(self.collisions.clone())
    }

    /// Update a single entity's state based on physics.
    fn update_entity_state(
        &self,
        entity_id: &EntityId,
        properties: &PhysicalProperties,
        state: &mut EntityState,
    ) -> Result<(), Error> {
        // Get accumulated forces
        let forces = self.forces.get(entity_id).unwrap_or(&Vec::new());
        
        // Sum all forces
        let mut total_force = (0.0, 0.0, 0.0);
        for &force in forces {
            total_force.0 += force.0;
            total_force.1 += force.1;
            total_force.2 += force.2;
        }

        // Apply drag force if entity has velocity
        if let Some(velocity) = state.velocity {
            let speed = (velocity.0 * velocity.0 + 
                        velocity.1 * velocity.1 + 
                        velocity.2 * velocity.2).sqrt();
            
            if speed > 0.0 && self.config.air_density > 0.0 {
                // Drag force: F_d = 0.5 * ρ * v² * C_d * A
                let drag_magnitude = 0.5 * self.config.air_density * 
                    speed * speed * 
                    properties.drag_coefficient * 
                    properties.cross_sectional_area;
                
                // Drag direction opposite to velocity
                let drag_force = (
                    -drag_magnitude * velocity.0 / speed,
                    -drag_magnitude * velocity.1 / speed,
                    -drag_magnitude * velocity.2 / speed,
                );
                
                total_force.0 += drag_force.0;
                total_force.1 += drag_force.1;
                total_force.2 += drag_force.2;
            }
        }

        // Acceleration: a = F / m
        let acceleration = (
            total_force.0 / properties.mass,
            total_force.1 / properties.mass,
            total_force.2 / properties.mass,
        );

        // Update velocity: v = v₀ + a * Δt
        if let Some(velocity) = state.velocity {
            state.velocity = Some((
                velocity.0 + acceleration.0 * self.config.time_step,
                velocity.1 + acceleration.1 * self.config.time_step,
                velocity.2 + acceleration.2 * self.config.time_step,
            ));
        } else {
            state.velocity = Some((
                acceleration.0 * self.config.time_step,
                acceleration.1 * self.config.time_step,
                acceleration.2 * self.config.time_step,
            ));
        }

        // Update position: x = x₀ + v * Δt
        if let Some(position) = state.position {
            if let Some(velocity) = state.velocity {
                state.position = Some((
                    position.0 + velocity.0 * self.config.time_step,
                    position.1 + velocity.1 * self.config.time_step,
                    position.2 + velocity.2 * self.config.time_step,
                ));
            }
        }

        // Update angular velocity if present
        if let Some(angular_velocity) = state.angular_velocity {
            // Simple damping
            let damping = 0.99;
            state.angular_velocity = Some((
                angular_velocity.0 * damping,
                angular_velocity.1 * damping,
                angular_velocity.2 * damping,
            ));
        }

        Ok(())
    }

    /// Detect collisions between entities.
    fn detect_collisions(
        &mut self,
        entity_states: &HashMap<EntityId, EntityState>,
    ) -> Result<(), Error> {
        let entity_ids: Vec<EntityId> = self.entities.keys().cloned().collect();
        
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                let id_a = entity_ids[i];
                let id_b = entity_ids[j];
                
                let prop_a = self.entities.get(&id_a).unwrap();
                let prop_b = self.entities.get(&id_b).unwrap();
                
                // Skip if both are static
                if prop_a.is_static && prop_b.is_static {
                    continue;
                }
                
                let state_a = entity_states.get(&id_a);
                let state_b = entity_states.get(&id_b);
                
                if let (Some(state_a), Some(state_b)) = (state_a, state_b) {
                    if let Some(collision) = self.check_collision(
                        &id_a, prop_a, state_a,
                        &id_b, prop_b, state_b,
                    ) {
                        self.collisions.push(collision);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Check for collision between two entities.
    fn check_collision(
        &self,
        id_a: &EntityId,
        prop_a: &PhysicalProperties,
        state_a: &EntityState,
        id_b: &EntityId,
        prop_b: &PhysicalProperties,
        state_b: &EntityState,
    ) -> Option<Collision> {
        // Get positions
        let pos_a = state_a.position?;
        let pos_b = state_b.position?;
        
        // Simple sphere-sphere collision for demonstration
        // In a real implementation, you would handle different shape types
        let radius_a = prop_a.collision_shape.bounding_radius();
        let radius_b = prop_b.collision_shape.bounding_radius();
        
        let dx = pos_b.0 - pos_a.0;
        let dy = pos_b.1 - pos_a.1;
        let dz = pos_b.2 - pos_a.2;
        
        let distance_sq = dx * dx + dy * dy + dz * dz;
        let min_distance = radius_a + radius_b;
        
        if distance_sq < min_distance * min_distance && distance_sq > 0.0 {
            let distance = distance_sq.sqrt();
            let normal = (dx / distance, dy / distance, dz / distance);
            let depth = min_distance - distance;
            
            // Calculate relative velocity
            let vel_a = state_a.velocity.unwrap_or((0.0, 0.0, 0.0));
            let vel_b = state_b.velocity.unwrap_or((0.0, 0.0, 0.0));
            let relative_velocity = (
                vel_b.0 - vel_a.0,
                vel_b.1 - vel_a.1,
                vel_b.2 - vel_a.2,
            );
            
            // Collision point (midpoint for simplicity)
            let point = (
                pos_a.0 + normal.0 * radius_a,
                pos_a.1 + normal.1 * radius_a,
                pos_a.2 + normal.2 * radius_a,
            );
            
            Some(Collision {
                entity_a: *id_a,
                entity_b: *id_b,
                point,
                normal,
                depth,
                relative_velocity,
            })
        } else {
            None
        }
    }

    /// Resolve a collision between two entities.
    fn resolve_collision(
        &self,
        collision: &Collision,
        entity_states: &mut HashMap<EntityId, EntityState>,
    ) -> Result<(), Error> {
        let prop_a = self.entities.get(&collision.entity_a)
            .ok_or_else(|| Error::Physics(format!("Entity {} not found", collision.entity_a)))?;
        let prop_b = self.entities.get(&collision.entity_b)
            .ok_or_else(|| Error::Physics(format!("Entity {} not found", collision.entity_b)))?;

        let state_a = entity_states.get_mut(&collision.entity_a)
            .ok_or_else(|| Error::Physics(format!("State for entity {} not found", collision.entity_a)))?;
        let state_b = entity_states.get_mut(&collision.entity_b)
            .ok_or_else(|| Error::Physics(format!("State for entity {} not found", collision.entity_b)))?;

        // Skip if both are static
        if prop_a.is_static && prop_b.is_static {
            return Ok(());
        }

        // Get velocities
        let vel_a = state_a.velocity.unwrap_or((0.0, 0.0, 0.0));
        let vel_b = state_b.velocity.unwrap_or((0.0, 0.0, 0.0));

        // Relative velocity along collision normal
        let rel_vel_normal = (
            collision.relative_velocity.0 * collision.normal.0 +
            collision.relative_velocity.1 * collision.normal.1 +
            collision.relative_velocity.2 * collision.normal.2
        );

        // Do not resolve if objects are moving apart
        if rel_vel_normal > 0.0 {
            return Ok(());
        }

        // Calculate impulse scalar
        let restitution = (prop_a.restitution + prop_b.restitution) / 2.0;
        let mut impulse_scalar = -(1.0 + restitution) * rel_vel_normal;
        
        let inv_mass_a = if prop_a.is_static { 0.0 } else { 1.0 / prop_a.mass };
        let inv_mass_b = if prop_b.is_static { 0.0 } else { 1.0 / prop_b.mass };
        
        impulse_scalar /= inv_mass_a + inv_mass_b;

        // Apply impulse
        let impulse = (
            impulse_scalar * collision.normal.0,
            impulse_scalar * collision.normal.1,
            impulse_scalar * collision.normal.2,
        );

        // Update velocities
        if !prop_a.is_static {
            let new_vel_a = (
                vel_a.0 - impulse.0 * inv_mass_a,
                vel_a.1 - impulse.1 * inv_mass_a,
                vel_a.2 - impulse.2 * inv_mass_a,
            );
            state_a.velocity = Some(new_vel_a);
        }

        if !prop_b.is_static {
            let new_vel_b = (
                vel_b.0 + impulse.0 * inv_mass_b,
                vel_b.1 + impulse.1 * inv_mass_b,
                vel_b.2 + impulse.2 * inv_mass_b,
            );
            state_b.velocity = Some(new_vel_b);
        }

        // Position correction to prevent sinking
        let correction_factor = 0.2; // Typically 0.2 to 0.8
        let correction = correction_factor * collision.depth / (inv_mass_a + inv_mass_b);
        
        let correction_a = (
            -correction * collision.normal.0 * inv_mass_a,
            -correction * collision.normal.1 * inv_mass_a,
            -correction * collision.normal.2 * inv_mass_a,
        );
        
        let correction_b = (
            correction * collision.normal.0 * inv_mass_b,
            correction * collision.normal.1 * inv_mass_b,
            correction * collision.normal.2 * inv_mass_b,
        );

        // Apply position correction
        if !prop_a.is_static {
            if let Some(pos) = state_a.position {
                state_a.position = Some((
                    pos.0 + correction_a.0,
                    pos.1 + correction_a.1,
                    pos.2 + correction_a.2,
                ));
            }
        }

        if !prop_b.is_static {
            if let Some(pos) = state_b.position {
                state_b.position = Some((
                    pos.0 + correction_b.0,
                    pos.1 + correction_b.1,
                    pos.2 + correction_b.2,
                ));
            }
        }

        Ok(())
    }

    /// Get the configuration.
    pub fn config(&self) -> &PhysicsConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut PhysicsConfig {
        &mut self.config
    }
}

/// Simple kinematic controller for moving entities along a path.
pub struct KinematicController {
    /// Target position.
    target_position: Option<(f64, f64, f64)>,
    /// Maximum speed (m/s).
    max_speed: f64,
    /// Arrival tolerance (meters).
    arrival_tolerance: f64,
    /// Whether the controller is active.
    is_active: bool,
}

impl KinematicController {
    /// Create a new kinematic controller.
    pub fn new(max_speed: f64, arrival_tolerance: f64) -> Self {
        Self {
            target_position: None,
            max_speed,
            arrival_tolerance,
            is_active: false,
        }
    }

    /// Set target position.
    pub fn set_target(&mut self, target: (f64, f64, f64)) {
        self.target_position = Some(target);
        self.is_active = true;
    }

    /// Clear target position.
    pub fn clear_target(&mut self) {
        self.target_position = None;
        self.is_active = false;
    }

    /// Update entity state to move toward target.
    pub fn update(
        &self,
        entity_state: &mut EntityState,
        physics_engine: &mut PhysicsEngine,
        dt: f64,
    ) -> Result<bool, Error> {
        if !self.is_active || self.target_position.is_none() {
            return Ok(false);
        }

        let target = self.target_position.unwrap();
        let current = entity_state.position.unwrap_or((0.0, 0.0, 0.0));

        // Calculate direction to target
        let dx = target.0 - current.0;
        let dy = target.1 - current.1;
        let dz = target.2 - current.2;

        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Check if arrived
        if distance < self.arrival_tolerance {
            return Ok(true);
        }

        // Normalize direction
        let direction = if distance > 0.0 {
            (dx / distance, dy / distance, dz / distance)
        } else {
            (0.0, 0.0, 0.0)
        };

        // Calculate desired velocity
        let desired_speed = self.max_speed.min(distance / dt);
        let desired_velocity = (
            direction.0 * desired_speed,
            direction.1 * desired_speed,
            direction.2 * desired_speed,
        );

        // Apply force to achieve desired velocity
        if let Some(current_velocity) = entity_state.velocity {
            // Calculate acceleration needed
            let acceleration = (
                (desired_velocity.0 - current_velocity.0) / dt,
                (desired_velocity.1 - current_velocity.1) / dt,
                (desired_velocity.2 - current_velocity.2) / dt,
            );

            // Get entity properties to calculate force
            if let Some(properties) = physics_engine.entities.get(&entity_state.entity_id) {
                let force = (
                    acceleration.0 * properties.mass,
                    acceleration.1 * properties.mass,
                    acceleration.2 * properties.mass,
                );

                physics_engine.apply_force(&entity_state.entity_id, force)?;
            }
        } else {
            // No current velocity, just set it
            entity_state.velocity = Some(desired_velocity);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_physics_engine_creation() {
        let config = PhysicsConfig::default();
        let engine = PhysicsEngine::new(config);
        
        assert_eq!(engine.config().gravity.2, -9.81);
        assert_eq!(engine.config().time_step, 0.01);
    }

    #[test]
    fn test_collision_shape_bounding_radius() {
        let sphere = CollisionShape::Sphere { radius: 2.0 };
        assert_eq!(sphere.bounding_radius(), 2.0);
        
        let box_shape = CollisionShape::Box { width: 2.0, height: 3.0, depth: 4.0 };
        let expected = (2.0 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0).sqrt() / 2.0;
        assert!((box_shape.bounding_radius() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_register_entity() {
        let mut engine = PhysicsEngine::new(PhysicsConfig::default());
        let entity_id = Uuid::new_v4();
        let properties = PhysicalProperties::default();
        
        assert!(engine.register_entity(entity_id, properties).is_ok());
        assert!(engine.register_entity(entity_id, PhysicalProperties::default()).is_err());
    }
}