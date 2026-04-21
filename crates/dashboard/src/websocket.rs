//! WebSocket manager for real-time dashboard updates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use crate::api::{TaskInfo, WorkflowInfo, AgentInfo};

/// WebSocket message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "agent_status")]
    AgentStatus { agent_id: String, status: AgentStatus },
    
    #[serde(rename = "task_created")]
    TaskCreated(TaskInfo),
    
    #[serde(rename = "task_updated")]
    TaskUpdated(TaskInfo),
    
    #[serde(rename = "task_completed")]
    TaskCompleted { task_id: String, result: String },
    
    #[serde(rename = "workflow_started")]
    WorkflowStarted(WorkflowInfo),
    
    #[serde(rename = "workflow_updated")]
    WorkflowUpdated(WorkflowInfo),
    
    #[serde(rename = "workflow_completed")]
    WorkflowCompleted { workflow_id: String, success: bool },
    
    #[serde(rename = "metrics_update")]
    MetricsUpdate(SystemMetrics),
    
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub state: String,
    pub battery_level: Option<f64>,
    pub connected_peers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub network_latency_ms: f64,
}

/// WebSocket client connection.
pub struct WsClient {
    pub tx: mpsc::UnboundedSender<WsMessage>,
    pub client_id: String,
}

/// WebSocket manager.
pub struct WebSocketManager {
    clients: RwLock<HashMap<String, mpsc::UnboundedSender<WsMessage>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }

    /// Add a new client connection.
    pub async fn add_client(&mut self, tx: mpsc::UnboundedSender<WsMessage>) -> String {
        let client_id = uuid::Uuid::new_v4().to_string();
        
        {
            let mut clients = self.clients.write().await;
            clients.insert(client_id.clone(), tx);
        }
        
        info!("WebSocket client connected: {}", client_id);
        client_id
    }

    /// Remove a client connection.
    pub async fn remove_client(&mut self, client_id: &str) {
        let mut clients = self.clients.write().await;
        if clients.remove(client_id).is_some() {
            info!("WebSocket client disconnected: {}", client_id);
        }
    }

    /// Broadcast a message to all connected clients.
    pub async fn broadcast(&mut self, message: WsMessage) {
        let clients = self.clients.read().await;
        let mut failed = Vec::new();

        for (client_id, tx) in clients.iter() {
            if let Err(e) = tx.send(message.clone()) {
                warn!("Failed to send message to client {}: {}", client_id, e);
                failed.push(client_id.clone());
            }
        }

        // Remove failed clients
        for client_id in failed {
            self.remove_client(&client_id).await;
        }
    }

    /// Broadcast to specific clients (e.g., by agent).
    pub async fn broadcast_to(
        &mut self,
        target_ids: &[String],
        message: WsMessage,
    ) {
        let clients = self.clients.read().await;

        for target_id in target_ids {
            if let Some(tx) = clients.get(target_id) {
                if let Err(e) = tx.send(message.clone()) {
                    warn!("Failed to send to {}: {}", target_id, e);
                }
            }
        }
    }

    // Convenience methods for common broadcast scenarios

    pub async fn broadcast_agent_status(&mut self, agent_id: &str, status: AgentStatus) {
        self.broadcast(WsMessage::AgentStatus {
            agent_id: agent_id.to_string(),
            status,
        })
        .await;
    }

    pub async fn broadcast_task_created(&mut self, task: &TaskInfo) {
        self.broadcast(WsMessage::TaskCreated(task.clone())).await;
    }

    pub async fn broadcast_task_updated(&mut self, task: &TaskInfo) {
        self.broadcast(WsMessage::TaskUpdated(task.clone())).await;
    }

    pub async fn broadcast_task_completed(&mut self, task_id: &str, result: &str) {
        self.broadcast(WsMessage::TaskCompleted {
            task_id: task_id.to_string(),
            result: result.to_string(),
        })
        .await;
    }

    pub async fn broadcast_workflow_started(&mut self, workflow: &WorkflowInfo) {
        self.broadcast(WsMessage::WorkflowStarted(workflow.clone())).await;
    }

    pub async fn broadcast_workflow_updated(&mut self, workflow: &WorkflowInfo) {
        self.broadcast(WsMessage::WorkflowUpdated(workflow.clone())).await;
    }

    pub async fn broadcast_workflow_completed(
        &mut self,
        workflow_id: &str,
        success: bool,
    ) {
        self.broadcast(WsMessage::WorkflowCompleted {
            workflow_id: workflow_id.to_string(),
            success,
        })
        .await;
    }

    pub async fn broadcast_metrics(&mut self, metrics: SystemMetrics) {
        self.broadcast(WsMessage::MetricsUpdate(metrics)).await;
    }

    pub async fn broadcast_error(&mut self, message: &str) {
        self.broadcast(WsMessage::Error {
            message: message.to_string(),
        })
        .await;
    }

    /// Get number of connected clients.
    pub async fn client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_manager() {
        let mut manager = WebSocketManager::new();

        let (tx, _rx) = mpsc::unbounded_channel();
        let client_id = manager.add_client(tx).await;

        assert_eq!(manager.client_count().await, 1);
        assert!(!client_id.is_empty());

        manager.broadcast_error("test error").await;

        manager.remove_client(&client_id).await;
        assert_eq!(manager.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_messages() {
        let mut manager = WebSocketManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_client(tx).await;

        let status = AgentStatus {
            agent_id: "agent-1".to_string(),
            state: "active".to_string(),
            battery_level: Some(85.0),
            connected_peers: 3,
        };

        manager.broadcast_agent_status("agent-1", status).await;

        let task = TaskInfo {
            id: "task-1".to_string(),
            description: "Test task".to_string(),
            status: "pending".to_string(),
            priority: 100,
            assigned_agent: None,
            progress: 0.0,
            created_at: 0,
            started_at: None,
            completed_at: None,
        };

        manager.broadcast_task_created(&task).await;
    }
}
