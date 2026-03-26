//! Extended demo with three agents synchronizing state via in‑memory transport.

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

    println!("Starting multi‑agent demo with in‑memory transport...");

    // Create three agents with in‑memory backend
    let mut agent1 = Agent::new(
        AgentId(1),
        MeshTransportConfig {
            local_agent_id: AgentId(1),
            static_peers: vec![],
            use_mdns: false,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            use_in_memory: true,
        },
    )?;

    let mut agent2 = Agent::new(
        AgentId(2),
        MeshTransportConfig {
            local_agent_id: AgentId(2),
            static_peers: vec![],
            use_mdns: false,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            use_in_memory: true,
        },
    )?;

    let mut agent3 = Agent::new(
        AgentId(3),
        MeshTransportConfig {
            local_agent_id: AgentId(3),
            static_peers: vec![],
            use_mdns: false,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            use_in_memory: true,
        },
    )?;

    // Start all agents
    agent1.start()?;
    agent2.start()?;
    agent3.start()?;

    println!("Agents started. Waiting for discovery...");
    sleep(Duration::from_secs(1)).await;

    // Agent1 sets a value
    println!("Agent 1 setting counter = 100");
    agent1.set_value("counter", json!(100))?;
    agent1.broadcast_changes().await?;
    println!("Agent 1 broadcast changes");

    // Wait for propagation
    sleep(Duration::from_millis(500)).await;

    // Agent2 should have received the update
    println!("Agent 2 checking counter...");
    if let Some(value) = agent2.get_value::<serde_json::Value>("counter") {
        println!("Agent 2 counter value: {}", value);
        assert_eq!(value, json!(100));
    } else {
        println!("Agent 2 did not receive the update");
    }

    // Agent3 also should have it
    println!("Agent 3 checking counter...");
    if let Some(value) = agent3.get_value::<serde_json::Value>("counter") {
        println!("Agent 3 counter value: {}", value);
        assert_eq!(value, json!(100));
    } else {
        println!("Agent 3 did not receive the update");
    }

    // Now agent2 updates the counter
    println!("Agent 2 incrementing counter by 5");
    if let Some(mut value) = agent2.get_value::<i64>("counter") {
        value += 5;
        agent2.set_value("counter", json!(value))?;
        agent2.broadcast_changes().await?;
        println!("Agent 2 broadcast changes");
    }

    sleep(Duration::from_millis(500)).await;

    // Check final values
    println!("Final values:");
    let v1 = agent1.get_value::<i64>("counter").unwrap_or(-1);
    let v2 = agent2.get_value::<i64>("counter").unwrap_or(-1);
    let v3 = agent3.get_value::<i64>("counter").unwrap_or(-1);
    println!("  Agent 1: {}", v1);
    println!("  Agent 2: {}", v2);
    println!("  Agent 3: {}", v3);

    // All should converge to 105
    assert_eq!(v1, 105);
    assert_eq!(v2, 105);
    assert_eq!(v3, 105);

    // Stop agents
    agent1.stop().await?;
    agent2.stop().await?;
    agent3.stop().await?;

    println!("Demo completed successfully.");
    Ok(())
}