//! Services for communicating with the backend.

use crate::models::{Agent, Metrics, Task};
use gloo_net::websocket::{futures::WebSocket, Message};
use serde_json;
use std::collections::VecDeque;
use yew::Callback;

/// WebSocket service for real‑time updates.
pub struct WebSocketService {
    ws: WebSocket,
    buffer: VecDeque<String>,
}

impl WebSocketService {
    /// Connect to the dashboard WebSocket endpoint.
    pub fn connect(url: &str) -> Result<Self, String> {
        let ws = WebSocket::open(url).map_err(|e| format!("WebSocket error: {}", e))?;
        Ok(Self {
            ws,
            buffer: VecDeque::new(),
        })
    }

    /// Send a command to the backend.
    pub fn send(&mut self, command: &str) -> Result<(), String> {
        self.ws
            .send(Message::Text(command.into()))
            .map_err(|e| format!("Send error: {}", e))
    }

    /// Poll for incoming messages.
    pub fn poll(&mut self) -> Option<String> {
        match self.ws.try_next() {
            Ok(Some(Message::Text(text))) => Some(text),
            Ok(Some(Message::Bytes(_))) => None,
            Ok(None) => None,
            Err(_) => None,
        }
    }

    /// Subscribe to agent updates.
    pub fn subscribe_agents(&mut self, callback: Callback<Vec<Agent>>) -> Result<(), String> {
        self.send(r#"{"type":"subscribe","topic":"agents"}"#)?;
        // In a real implementation, we would spawn a task to forward messages.
        Ok(())
    }

    /// Subscribe to task updates.
    pub fn subscribe_tasks(&mut self, callback: Callback<Vec<Task>>) -> Result<(), String> {
        self.send(r#"{"type":"subscribe","topic":"tasks"}"#)?;
        Ok(())
    }

    /// Subscribe to metrics updates.
    pub fn subscribe_metrics(&mut self, callback: Callback<Metrics>) -> Result<(), String> {
        self.send(r#"{"type":"subscribe","topic":"metrics"}"#)?;
        Ok(())
    }
}

/// Mock service for development.
pub struct MockService;

impl MockService {
    /// Get mock agents.
    pub fn get_agents() -> Vec<Agent> {
        vec![
            Agent {
                id: "agent-001".to_string(),
                capabilities: vec!["compute".to_string(), "storage".to_string()],
                state: crate::models::AgentState::Running,
                resources: crate::models::ResourceUsage {
                    cpu_percent: 45.2,
                    memory_percent: 67.8,
                    disk_percent: 23.1,
                },
                last_heartbeat: 1711887600,
            },
            Agent {
                id: "agent-002".to_string(),
                capabilities: vec!["sensor".to_string()],
                state: crate::models::AgentState::Pending,
                resources: crate::models::ResourceUsage {
                    cpu_percent: 10.0,
                    memory_percent: 30.0,
                    disk_percent: 50.0,
                },
                last_heartbeat: 1711887500,
            },
        ]
    }

    /// Get mock tasks.
    pub fn get_tasks() -> Vec<Task> {
        vec![
            Task {
                id: "task-001".to_string(),
                description: "Process sensor data".to_string(),
                assigned_agent: Some("agent-001".to_string()),
                status: crate::models::TaskStatus::Running,
                priority: 5,
                deadline: Some(1711890000),
            },
            Task {
                id: "task-002".to_string(),
                description: "Backup database".to_string(),
                assigned_agent: None,
                status: crate::models::TaskStatus::Pending,
                priority: 3,
                deadline: None,
            },
        ]
    }

    /// Get mock metrics.
    pub fn get_metrics() -> Metrics {
        Metrics {
            total_agents: 2,
            total_tasks: 5,
            tasks_completed: 3,
            tasks_failed: 1,
            network_latency_ms: 12.5,
            message_rate: 42.7,
        }
    }
}