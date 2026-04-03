//! Federated learning with distributed coordination demo.
//!
//! This example shows how to use the enhanced federated learning crate
//! with distributed coordination across multiple agents.

use std::sync::Arc;
use tokio::sync::mpsc;
use federated_learning::prelude::*;
use federated_learning::model::{Model, Layer, LayerType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Federated Learning with Distributed Coordination Demo ===");
    
    // Create a simple neural network model
    let model = Model {
        name: "demo-model".to_string(),
        layers: vec![
            Layer {
                layer_type: LayerType::Dense(128),
                activation: Some("relu".to_string()),
                trainable: true,
            },
            Layer {
                layer_type: LayerType::Dense(64),
                activation: Some("relu".to_string()),
                trainable: true,
            },
            Layer {
                layer_type: LayerType::Dense(10),
                activation: Some("softmax".to_string()),
                trainable: true,
            },
        ],
        parameter_count: 128 * 64 + 64 * 10, // simplified
        version: 1,
        metadata: std::collections::HashMap::new(),
    };
    
    // Create event channel for monitoring
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    // Create distributed training configuration
    let config = DistributedTrainingConfig {
        min_agents: 2,
        max_agents: Some(5),
        max_rounds: Some(3),
        round_timeout_secs: 60,
        aggregation: AggregationConfig::default(),
        enable_privacy: true,
        checkpoint_interval: Some(1),
    };
    
    // Create coordinator
    let coordinator = Arc::new(DistributedTrainingCoordinator::new(
        config,
        model,
        event_tx,
    )?);
    
    println!("✓ Created distributed training coordinator");
    
    // Start event listener task
    let coordinator_clone = coordinator.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                DistributedTrainingEvent::RoundStarted { round_id, participants, model_version } => {
                    println!("[EVENT] Round {} started with {} participants (model v{})", 
                             round_id, participants, model_version);
                }
                DistributedTrainingEvent::UpdateReceived { client_id, round_id, samples } => {
                    println!("[EVENT] Update from {} for round {} ({} samples)", 
                             client_id, round_id, samples);
                }
                DistributedTrainingEvent::AggregationCompleted { round_id, aggregated_model_version, participant_count } => {
                    println!("[EVENT] Round {} aggregated (model v{}, {} participants)", 
                             round_id, aggregated_model_version, participant_count);
                }
                DistributedTrainingEvent::RoundFailed { round_id, reason } => {
                    println!("[EVENT] Round {} failed: {}", round_id, reason);
                }
            }
        }
    });
    
    // Simulate registering participants
    println!("\n=== Registering Participants ===");
    for i in 1..=3 {
        let agent_id = format!("agent-{}", i);
        // In a real scenario, you'd create actual FederatedClient instances
        // For demo, we'll use placeholder
        // let client = FederatedClient::new(...);
        // coordinator.register_participant(agent_id, client).await?;
        println!("  Registered {}", agent_id);
    }
    
    // Get initial stats
    let stats = coordinator.get_stats().await;
    println!("\n=== Initial Stats ===");
    println!("Round: {}", stats.round);
    println!("Active participants: {}", stats.active_participants);
    println!("Model version: {}", stats.model_version);
    
    // Start a training round
    println!("\n=== Starting Training Round ===");
    let round_started = coordinator.start_round().await?;
    if round_started {
        println!("Round started successfully");
    } else {
        println!("Not enough participants to start round");
    }
    
    // Simulate receiving updates (in a real scenario, these would come from agents)
    println!("\n=== Simulating Updates ===");
    for i in 1..=2 {
        let agent_id = format!("agent-{}", i);
        // Create a mock update
        let update = ClientUpdate {
            client_id: agent_id.clone(),
            round_id: 1,
            parameters: vec![0.1, 0.2, 0.3], // dummy parameters
            sample_count: 100 * i,
            metadata: std::collections::HashMap::new(),
        };
        
        // In a real implementation, you'd call:
        // coordinator.receive_update(&agent_id, update).await?;
        println!("  Simulated update from {} ({} samples)", agent_id, update.sample_count);
    }
    
    // Get updated stats
    let stats = coordinator.get_stats().await;
    println!("\n=== Updated Stats ===");
    println!("Round: {}", stats.round);
    println!("Total updates: {}", stats.total_updates);
    
    // Demonstrate mesh integration
    println!("\n=== Mesh Integration ===");
    let mesh_integration = MeshFederatedIntegration::new(coordinator.clone());
    println!("Created mesh integration");
    
    // Simulate handling a mesh message
    let mock_message = b"mock federated learning message";
    mesh_integration.handle_message("agent-1", mock_message).await?;
    println!("Handled mock mesh message");
    
    // Stop event listener
    drop(coordinator); // This will cause the event channel to close
    let _ = event_task.await;
    
    println!("\n=== Demo Complete ===");
    Ok(())
}