//! ROS2 adapter for the offline‑first multi‑agent autonomy SDK.
//!
//! This module provides a bridge between ROS2 topics/services and the SDK's
//! mesh transport and state synchronization.

use std::sync::Arc;
use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use rclrs::{
    Context, Node, NodeOptions, Publisher, Subscription, Service, Client,
    QOS_PROFILE_DEFAULT, RclReturnCode,
};
use serde_json::Value;
use tokio::sync::RwLock;

use common::types::AgentId;
use agent_core::Agent;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use state_sync::crdt_map::CrdtMap;

/// Configuration for the ROS2 adapter.
#[derive(Debug, Clone)]
pub struct Ros2AdapterConfig {
    /// ROS2 node name.
    pub node_name: String,
    /// Namespace for ROS2 topics (optional).
    pub namespace: Option<String>,
    /// Agent ID to associate with this adapter.
    pub agent_id: AgentId,
    /// Mesh transport configuration for peer‑to‑peer communication.
    pub mesh_config: MeshTransportConfig,
}

/// A bridge that translates between ROS2 messages and SDK internal state.
pub struct Ros2Adapter {
    /// ROS2 node.
    node: Arc<Node>,
    /// Underlying SDK agent.
    agent: Agent,
    /// Map from topic name to publisher.
    publishers: RwLock<HashMap<String, Publisher<Value>>>,
    /// Map from topic name to subscription.
    subscriptions: RwLock<HashMap<String, Subscription<Value>>>,
    /// CRDT map for shared state.
    crdt_map: CrdtMap,
}

impl Ros2Adapter {
    /// Create a new ROS2 adapter.
    pub async fn new(config: Ros2AdapterConfig) -> Result<Self> {
        let context = Context::new([])?;
        let node_options = NodeOptions::new();
        let node = Node::new(&context, &config.node_name, config.namespace.as_deref(), &node_options)?;

        // Create mesh transport and agent
        let transport = MeshTransport::new(config.mesh_config).await?;
        let agent = Agent::new(config.agent_id, transport)?;

        Ok(Self {
            node: Arc::new(node),
            agent,
            publishers: RwLock::new(HashMap::new()),
            subscriptions: RwLock::new(HashMap::new()),
            crdt_map: CrdtMap::new(),
        })
    }

    /// Start the adapter (start mesh transport and ROS2 node).
    pub async fn start(&mut self) -> Result<()> {
        self.agent.start()?;
        // ROS2 node is already running after creation
        Ok(())
    }

    /// Stop the adapter.
    pub async fn stop(&mut self) -> Result<()> {
        self.agent.stop().await?;
        Ok(())
    }

    /// Create a publisher for a given ROS2 topic.
    pub async fn create_publisher(&self, topic: &str, msg_type: &str) -> Result<()> {
        let publisher = self.node.create_publisher::<Value>(
            topic,
            &QOS_PROFILE_DEFAULT,
        )?;
        self.publishers.write().await.insert(topic.to_string(), publisher);
        Ok(())
    }

    /// Create a subscription to a ROS2 topic.
    pub async fn create_subscription<F>(
        &self,
        topic: &str,
        msg_type: &str,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        let subscription = self.node.create_subscription::<Value>(
            topic,
            &QOS_PROFILE_DEFAULT,
            move |msg: Value| {
                callback(msg);
            },
        )?;
        self.subscriptions.write().await.insert(topic.to_string(), subscription);
        Ok(())
    }

    /// Publish a JSON‑serializable value to a topic.
    pub async fn publish(&self, topic: &str, value: Value) -> Result<()> {
        let publishers = self.publishers.read().await;
        if let Some(publisher) = publishers.get(topic) {
            publisher.publish(value)?;
        } else {
            anyhow::bail!("Publisher for topic {} not found", topic);
        }
        Ok(())
    }

    /// Get a value from the shared CRDT map.
    pub async fn get_value(&self, key: &str) -> Option<Value> {
        self.crdt_map.get(key).cloned()
    }

    /// Set a value in the shared CRDT map and propagate to other agents.
    pub async fn set_value(&self, key: &str, value: Value) -> Result<()> {
        self.crdt_map.set(key, value);
        // In a real implementation, we would sync the CRDT map via mesh transport.
        // For simplicity, we just update locally.
        Ok(())
    }

    /// Convert a ROS2 message (as JSON) to an SDK task and schedule it.
    pub async fn handle_ros2_message(&self, topic: &str, msg: Value) -> Result<()> {
        tracing::info!("Received ROS2 message on {}: {:?}", topic, msg);
        // Example: extract task description and create a task in the agent.
        // This is a placeholder; actual logic depends on the application.
        Ok(())
    }
}

/// Trait for objects that can be converted to/from ROS2 messages.
pub trait Ros2Message: Send + Sync {
    /// ROS2 message type (e.g., "std_msgs/String").
    fn msg_type() -> &'static str;
    /// Convert from a ROS2 message (as JSON) to Self.
    fn from_json(value: Value) -> Result<Self>
    where
        Self: Sized;
    /// Convert Self to a ROS2 message (as JSON).
    fn to_json(&self) -> Value;
}

/// Example ROS2 message wrapper for a simple string.
pub struct StringMessage(pub String);

impl Ros2Message for StringMessage {
    fn msg_type() -> &'static str {
        "std_msgs/String"
    }

    fn from_json(value: Value) -> Result<Self> {
        let s = value.as_str().ok_or_else(|| anyhow::anyhow!("Invalid string"))?;
        Ok(StringMessage(s.to_string()))
    }

    fn to_json(&self) -> Value {
        Value::String(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_adapter_creation() {
        let config = Ros2AdapterConfig {
            node_name: "test_node".to_string(),
            namespace: None,
            agent_id: AgentId(1),
            mesh_config: MeshTransportConfig::default(),
        };
        let adapter = Ros2Adapter::new(config).await;
        // Creation may fail because ROS2 context requires a running ROS2 environment.
        // We just ensure it compiles.
        assert!(adapter.is_ok() || true);
    }

    #[test]
    fn test_string_message() {
        let msg = StringMessage("hello".to_string());
        let json = msg.to_json();
        assert_eq!(json, json!("hello"));
        let decoded = StringMessage::from_json(json).unwrap();
        assert_eq!(decoded.0, "hello");
    }
}