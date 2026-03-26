//! Integration tests for mesh transport.

use mesh_transport::{MeshTransport, MeshTransportConfig};
use common::types::AgentId;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_two_agents_communication() {
    let config1 = MeshTransportConfig {
        local_agent_id: AgentId(1),
        static_peers: vec![],
        use_mdns: false,
        listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        use_in_memory: true,
    };
    let config2 = MeshTransportConfig {
        local_agent_id: AgentId(2),
        static_peers: vec![],
        use_mdns: false,
        listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        use_in_memory: true,
    };

    let mut transport1 = MeshTransport::new(config1).await.expect("Failed to create transport1");
    let mut transport2 = MeshTransport::new(config2).await.expect("Failed to create transport2");

    transport1.start().await.expect("Failed to start transport1");
    transport2.start().await.expect("Failed to start transport2");

    // Wait a bit for initialization
    sleep(Duration::from_millis(50)).await;

    // Send a message from 1 to 2
    let payload = b"Hello from agent 1".to_vec();
    transport1.send_to(AgentId(2), payload.clone()).await.expect("Send failed");

    // Check that transport2 receives the message
    let mut events = transport2.events();
    let received = tokio::time::timeout(Duration::from_millis(500), events.next()).await;
    assert!(received.is_ok(), "Timeout waiting for message");
    if let Some(Ok(event)) = received.unwrap() {
        match event {
            mesh_transport::TransportEvent::MessageReceived { from, payload: p } => {
                assert_eq!(from, AgentId(1));
                assert_eq!(p, payload);
            }
            _ => panic!("Unexpected event: {:?}", event),
        }
    } else {
        panic!("No event received");
    }

    // Broadcast test
    let broadcast_payload = b"Broadcast message".to_vec();
    transport1.broadcast(broadcast_payload.clone()).await.expect("Broadcast failed");

    let mut events2 = transport2.events();
    let received2 = tokio::time::timeout(Duration::from_millis(500), events2.next()).await;
    assert!(received2.is_ok(), "Timeout waiting for broadcast");
    if let Some(Ok(event)) = received2.unwrap() {
        match event {
            mesh_transport::TransportEvent::MessageReceived { from, payload: p } => {
                assert_eq!(from, AgentId(1));
                assert_eq!(p, broadcast_payload);
            }
            _ => panic!("Unexpected event: {:?}", event),
        }
    } else {
        panic!("No broadcast event received");
    }

    // Stop transports
    transport1.stop().await.expect("Failed to stop transport1");
    transport2.stop().await.expect("Failed to stop transport2");
}

#[tokio::test]
async fn test_peer_discovery() {
    // In‑memory backend does not have peer discovery, but we can test that peers() returns empty.
    let config = MeshTransportConfig {
        local_agent_id: AgentId(42),
        static_peers: vec![],
        use_mdns: false,
        listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        use_in_memory: true,
    };
    let transport = MeshTransport::new(config).await.expect("Failed to create transport");
    let peers = transport.peers();
    assert!(peers.is_empty());
}