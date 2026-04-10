//! Digital twin demonstration.
//!
//! This example shows how to create a digital twin environment with entities,
//! simulate physics, and visualize the scene.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use digital_twin::{
    Camera, CollisionShape, DigitalTwinModel, Entity, EntityState, EntityType,
    OperationalStatus, PhysicsConfig, PhysicsEngine, PhysicalProperties,
    Projection, Scene, Simple2DRenderer, VisualizationManager,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Digital Twin Demo ===");

    // 1. Create a digital twin model
    println!("1. Creating digital twin model...");
    let mut model = DigitalTwinModel::new("Factory Floor");

    // Add entities
    let robot_id = Uuid::new_v4();
    let sensor_id = Uuid::new_v4();
    let machine_id = Uuid::new_v4();

    let robot = Entity::new(robot_id, EntityType::Robot, "Robot-001");
    let sensor = Entity::new(sensor_id, EntityType::Sensor, "Temperature-Sensor-01");
    let machine = Entity::new(machine_id, EntityType::Machine, "CNC-Machine-05");

    model.add_entity(robot);
    model.add_entity(sensor);
    model.add_entity(machine);

    // Add a relationship
    model.add_relationship(
        robot_id,
        machine_id,
        "controls".to_string(),
        None,
    );

    println!("   Model created with {} entities, {} relationships",
             model.entities().len(), model.relationships().len());

    // 2. Create entity states
    println!("2. Creating entity states...");
    let robot_state = EntityState {
        entity_id: robot_id,
        timestamp: Utc::now(),
        position: Some((0.0, 0.0, 0.5)),
        orientation: Some((0.0, 0.0, 0.0, 1.0)),
        velocity: Some((0.1, 0.0, 0.0)),
        angular_velocity: Some((0.0, 0.0, 0.05)),
        status: OperationalStatus::Operational,
        battery_level: Some(0.85),
        temperature: Some(35.5),
        custom_state: Default::default(),
    };

    let sensor_state = EntityState {
        entity_id: sensor_id,
        timestamp: Utc::now(),
        position: Some((2.0, 1.5, 1.0)),
        orientation: Some((0.0, 0.0, 0.0, 1.0)),
        velocity: Some((0.0, 0.0, 0.0)),
        angular_velocity: None,
        status: OperationalStatus::Operational,
        battery_level: Some(0.95),
        temperature: Some(22.5),
        custom_state: Default::default(),
    };

    let machine_state = EntityState {
        entity_id: machine_id,
        timestamp: Utc::now(),
        position: Some((1.5, -1.0, 0.0)),
        orientation: Some((0.0, 0.0, 0.0, 1.0)),
        velocity: Some((0.0, 0.0, 0.0)),
        angular_velocity: None,
        status: OperationalStatus::Maintenance,
        battery_level: None,
        temperature: Some(45.0),
        custom_state: Default::default(),
    };

    // 3. Physics simulation
    println!("3. Setting up physics simulation...");
    let physics_config = PhysicsConfig::default();
    let mut physics_engine = PhysicsEngine::new(physics_config);

    // Add physical properties for robot
    let robot_props = PhysicalProperties {
        mass: 50.0,
        collision_shape: CollisionShape::Box {
            width: 0.8,
            height: 1.2,
            depth: 0.6,
        },
        friction: 0.7,
        restitution: 0.3,
        drag_coefficient: 0.5,
        buoyancy: 0.0,
    };
    physics_engine.add_entity(robot_id, robot_props, robot_state.clone());

    // Add sensor (static)
    let sensor_props = PhysicalProperties {
        mass: 0.5,
        collision_shape: CollisionShape::Sphere { radius: 0.1 },
        friction: 0.2,
        restitution: 0.1,
        drag_coefficient: 0.8,
        buoyancy: 0.0,
    };
    physics_engine.add_entity(sensor_id, sensor_props, sensor_state.clone());

    // Add machine (static)
    let machine_props = PhysicalProperties {
        mass: 500.0,
        collision_shape: CollisionShape::Box {
            width: 2.0,
            height: 1.5,
            depth: 1.0,
        },
        friction: 0.9,
        restitution: 0.1,
        drag_coefficient: 1.2,
        buoyancy: 0.0,
    };
    physics_engine.add_entity(machine_id, machine_props, machine_state.clone());

    println!("   Physics engine ready with {} entities", physics_engine.entity_count());

    // 4. Visualization
    println!("4. Setting up visualization...");
    let renderer = Simple2DRenderer::new(800, 600);
    let mut viz = VisualizationManager::new(renderer);
    viz.init()?;

    // Add entities to visualization scene
    let robot_entity = model.get_entity(robot_id).unwrap();
    let sensor_entity = model.get_entity(sensor_id).unwrap();
    let machine_entity = model.get_entity(machine_id).unwrap();

    viz.add_entity(robot_entity, &robot_state);
    viz.add_entity(sensor_entity, &sensor_state);
    viz.add_entity(machine_entity, &machine_state);

    // Configure camera
    viz.camera_mut().move_to(5.0, 5.0, 5.0);
    viz.camera_mut().look_at(0.0, 0.0, 0.0);
    viz.camera_mut().viewport_width = 800;
    viz.camera_mut().viewport_height = 600;

    // 5. Simulation loop
    println!("5. Running simulation loop (5 steps)...");
    for step in 0..5 {
        println!("   Step {}:", step + 1);

        // Update physics
        physics_engine.step(0.1); // 100 ms time step
        let updated_states = physics_engine.get_states();

        // Update visualization
        viz.update_scene(&updated_states);
        viz.render_frame()?;

        // Capture and show frame info (just metadata)
        let frame = viz.capture_frame()?;
        println!("     Frame size: {} bytes ({}x{} RGBA)",
                 frame.len(),
                 viz.camera().viewport_width,
                 viz.camera().viewport_height);

        // Print entity positions
        for state in &updated_states {
            if let Some(pos) = state.position {
                println!("     Entity {} at ({:.2}, {:.2}, {:.2})",
                         state.entity_id, pos.0, pos.1, pos.2);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 6. Cleanup
    println!("6. Cleaning up...");
    viz.stop()?;

    println!("=== Demo completed successfully ===");
    Ok(())
}