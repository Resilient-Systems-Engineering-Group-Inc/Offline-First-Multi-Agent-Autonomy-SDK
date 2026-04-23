//! Physics engine integration.

use anyhow::Result;
use nalgebra::{Vector3, Quaternion};
use serde::{Deserialize, Serialize};

/// Physics state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsState {
    pub timestamp: f64,
    pub gravity: Vector3<f64>,
    pub bodies: Vec<PhysicsBody>,
}

/// Physics body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsBody {
    pub id: String,
    pub position: Vector3<f64>,
    pub orientation: Quaternion<f64>,
    pub linear_velocity: Vector3<f64>,
    pub angular_velocity: Vector3<f64>,
    pub mass: f64,
    pub inertia: Vector3<f64>,
}

/// Collision detection.
pub struct CollisionDetector {
    enabled: bool,
}

impl CollisionDetector {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Check collision between two bodies.
    pub fn check_collision(&self, body1: &PhysicsBody, body2: &PhysicsBody) -> bool {
        if !self.enabled {
            return false;
        }

        // Simple sphere collision
        let distance = (body1.position - body2.position).norm();
        let min_distance = 0.5; // meters

        distance < min_distance
    }

    /// Check all collisions.
    pub fn check_all_collisions(&self, bodies: &[PhysicsBody]) -> Vec<(String, String)> {
        let mut collisions = vec![];

        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                if self.check_collision(&bodies[i], &bodies[j]) {
                    collisions.push((bodies[i].id.clone(), bodies[j].id.clone()));
                }
            }
        }

        collisions
    }
}

/// Physics engine.
pub struct PhysicsEngine {
    gravity: Vector3<f64>,
    dt: f64,
    collision_detector: CollisionDetector,
}

impl PhysicsEngine {
    pub fn new(gravity: [f64; 3], dt: f64, enable_collisions: bool) -> Self {
        Self {
            gravity: Vector3::from(gravity),
            dt,
            collision_detector: CollisionDetector::new(enable_collisions),
        }
    }

    /// Update physics state.
    pub fn update(&self, bodies: &mut [PhysicsBody]) -> PhysicsState {
        let mut physics_bodies = vec![];

        for body in bodies.iter_mut() {
            // Apply gravity
            body.linear_velocity += self.gravity * self.dt;

            // Update position
            body.position += body.linear_velocity * self.dt;

            // Update orientation
            let omega = body.angular_velocity * self.dt;
            let delta_q = Quaternion::new(
                1.0 - 0.25 * omega.norm_squared(),
                omega[0] * 0.5,
                omega[1] * 0.5,
                omega[2] * 0.5,
            ).normalize();
            
            body.orientation = (body.orientation * delta_q).normalize();

            physics_bodies.push(body.clone());
        }

        PhysicsState {
            timestamp: chrono::Utc::now().timestamp(),
            gravity: self.gravity,
            bodies: physics_bodies,
        }
    }

    /// Apply force to body.
    pub fn apply_force(&self, body: &mut PhysicsBody, force: Vector3<f64>) {
        let acceleration = force / body.mass;
        body.linear_velocity += acceleration * self.dt;
    }

    /// Apply torque to body.
    pub fn apply_torque(&self, body: &mut PhysicsBody, torque: Vector3<f64>) {
        let angular_acceleration = torque.component_div(&body.inertia);
        body.angular_velocity += angular_acceleration * self.dt;
    }

    /// Get collision detector.
    pub fn collision_detector(&self) -> &CollisionDetector {
        &self.collision_detector
    }
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self::new([0.0, 0.0, -9.81], 0.01, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_engine() {
        let engine = PhysicsEngine::default();

        let mut body = PhysicsBody {
            id: "test".to_string(),
            position: Vector3::new(0.0, 0.0, 10.0),
            orientation: Quaternion::identity(),
            linear_velocity: Vector3::zero(),
            angular_velocity: Vector3::zero(),
            mass: 1.0,
            inertia: Vector3::new(1.0, 1.0, 1.0),
        };

        // Apply gravity for 1 second
        for _ in 0..100 {
            engine.update(&mut [body.clone()]);
        }

        // Check position (should be lower due to gravity)
        assert!(body.position[2] < 10.0);
    }

    #[test]
    fn test_collision_detection() {
        let detector = CollisionDetector::new(true);

        let body1 = PhysicsBody {
            id: "body1".to_string(),
            position: Vector3::new(0.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            linear_velocity: Vector3::zero(),
            angular_velocity: Vector3::zero(),
            mass: 1.0,
            inertia: Vector3::new(1.0, 1.0, 1.0),
        };

        let body2 = PhysicsBody {
            id: "body2".to_string(),
            position: Vector3::new(0.3, 0.0, 0.0), // Close to body1
            orientation: Quaternion::identity(),
            linear_velocity: Vector3::zero(),
            angular_velocity: Vector3::zero(),
            mass: 1.0,
            inertia: Vector3::new(1.0, 1.0, 1.0),
        };

        assert!(detector.check_collision(&body1, &body2));

        let body3 = PhysicsBody {
            id: "body3".to_string(),
            position: Vector3::new(5.0, 0.0, 0.0), // Far from body1
            orientation: Quaternion::identity(),
            linear_velocity: Vector3::zero(),
            angular_velocity: Vector3::zero(),
            mass: 1.0,
            inertia: Vector3::new(1.0, 1.0, 1.0),
        };

        assert!(!detector.check_collision(&body1, &body3));
    }
}
