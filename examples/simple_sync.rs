//! Simple example of two agents synchronizing a counter.

use agent_core::Agent;
use common::types::AgentId;
use mesh_transport::MeshTransportConfig;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create two agents with different IDs
    let mut agent1 = Agent::new(
        AgentId(1),
        MeshTransportConfig {
            local_agent_id: AgentId(1),
            static_peers: vec![],
            use_mdns: true,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        },
    )?;

    let mut agent2 = Agent::new(
        AgentId(2),
        MeshTransportConfig {
            local_agent_id: AgentId(2),
            static_peers: vec![],
            use_mdns: true,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        },
    )?;

    // Start both agents
    agent1.start()?;
    agent2.start()?;

    println!("Agents started. Waiting for discovery...");
    sleep(Duration::from_secs(2)).await;

    // Agent1 sets a value in its CRDT map
    println!("Agent 1 setting counter = 42");
    agent1.set_value("counter", json!(42))?;

    // Broadcast changes
    agent1.broadcast_changes().await?;
    println!("Agent 1 broadcast changes");

    // Wait a bit for synchronization
    sleep(Duration::from_secs(1)).await;

    // Agent2 should have received the update
    println!("Agent 2 checking counter...");
    if let Some(value) = agent2.get_value::<serde_json::Value>("counter") {
        println!("Agent 2 counter value: {}", value);
        assert_eq!(value, json!(42));
    } else {
        println!("Agent 2 did not receive the update (transport not yet functional)");
    }

    // Stop agents
    agent1.stop().await?;
    agent2.stop().await?;

    println!("Example completed.");
    Ok(())
}