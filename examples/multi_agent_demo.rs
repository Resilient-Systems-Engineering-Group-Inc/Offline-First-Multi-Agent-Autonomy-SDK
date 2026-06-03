//! Extended demo with three agents synchronizing state via in‑memory transport.
//!
//! This example demonstrates:
//! - Creating multiple agents with the in-memory backend
//! - Cross-agent state synchronization via CRDT
//! - Concurrent value setting and broadcasting
//! - Proper async initialization and shutdown

use agent_core::Agent;
use common::types::AgentId;
use mesh_transport::{MeshTransportConfig, BackendType, SecurityMode};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting multi‑agent demo with in‑memory transport...");

    // Create three agents with in-memory backend
    let mut agent1 = Agent::new(
        AgentId(1),
        MeshTransportConfig {
            local_agent_id: AgentId(1),
            static_peers: vec![],
            use_mdns: false,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            backend_type: BackendType::InMemory,
            security_mode: SecurityMode::Classical,
            webrtc_config: None,
            lora_config: None,
        },
    ).await?;

    let mut agent2 = Agent::new(
        AgentId(2),
        MeshTransportConfig {
            local_agent_id: AgentId(2),
            static_peers: vec![],
            use_mdns: false,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            backend_type: BackendType::InMemory,
            security_mode: SecurityMode::Classical,
            webrtc_config: None,
            lora_config: None,
        },
    ).await?;

    let mut agent3 = Agent::new(
        AgentId(3),
        MeshTransportConfig {
            local_agent_id: AgentId(3),
            static_peers: vec![],
            use_mdns: false,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            backend_type: BackendType::InMemory,
            security_mode: SecurityMode::Classical,
            webrtc_config: None,
            lora_config: None,
        },
    ).await?;

    // Start all agents
    agent1.start()?;
    agent2.start()?;
    agent3.start()?;

    println!("All agents started. Waiting for discovery...");
    sleep(Duration::from_secs(1)).await;

    // Agent1 sets a value
    println!("Agent 1 setting role = leader");
    agent1.set_value("role", json!("leader"))?;
    agent1.broadcast_changes().await?;

    // Agent2 sets a different value
    println!("Agent 2 setting role = follower");
    agent2.set_value("role", json!("follower"))?;
    agent2.broadcast_changes().await?;

    // Agent3 sets a value
    println!("Agent 3 setting counter = 100");
    agent3.set_value("counter", json!(100))?;
    agent3.broadcast_changes().await?;

    // Wait for synchronization
    sleep(Duration::from_millis(500)).await;

    // Check values on each agent
    println!("\n=== Final State ===");
    for (i, agent) in [&agent1, &agent2, &agent3].iter().enumerate() {
        let role = agent.get_value::<serde_json::Value>("role");
        let counter = agent.get_value::<serde_json::Value>("counter");
        println!(
            "Agent {}: role={:?}, counter={:?}",
            i + 1,
            role,
            counter
        );
    }

    // Stop agents
    agent1.stop().await?;
    agent2.stop().await?;
    agent3.stop().await?;

    println!("\nMulti‑agent demo completed successfully.");
    Ok(())
}