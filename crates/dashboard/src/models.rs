//! Data models for the dashboard.

use serde::{Deserialize, Serialize};

/// Agent representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub capabilities: Vec<String>,
    pub state: AgentState,
    pub resources: ResourceUsage,
    pub last_heartbeat: u64,
}

/// Agent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentState {
    Pending,
    Running,
    Error,
    Terminated,
}

/// Resource usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub disk_percent: f64,
}

/// Task representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub assigned_agent: Option<String>,
    pub status: TaskStatus,
    pub priority: i32,
    pub deadline: Option<u64>,
}

/// Task status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

/// Network node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub address: String,
    pub connections: Vec<String>,
}

/// Metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub total_agents: usize,
    pub total_tasks: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub network_latency_ms: f64,
    pub message_rate: f64,
}