//! Debug session and command definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// A debug session attached to an agent or a group of agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    /// Unique session ID.
    pub id: Uuid,
    /// Agent IDs being debugged.
    pub agent_ids: Vec<crate::common::types::AgentId>,
    /// Session start timestamp.
    pub started_at: u64,
    /// Session end timestamp (if finished).
    pub ended_at: Option<u64>,
    /// Session metadata.
    pub metadata: HashMap<String, String>,
    /// Collected logs.
    pub logs: Vec<LogEntry>,
    /// Collected metrics.
    pub metrics: Vec<MetricSnapshot>,
    /// Breakpoints.
    pub breakpoints: Vec<Breakpoint>,
}

impl DebugSession {
    /// Create a new debug session.
    pub fn new(agent_ids: Vec<crate::common::types::AgentId>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: Uuid::new_v4(),
            agent_ids,
            started_at: now,
            ended_at: None,
            metadata: HashMap::new(),
            logs: Vec::new(),
            metrics: Vec::new(),
            breakpoints: Vec::new(),
        }
    }

    /// End the session.
    pub fn end(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.ended_at = Some(now);
    }

    /// Add a log entry.
    pub fn add_log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
    }

    /// Add a metric snapshot.
    pub fn add_metric(&mut self, metric: MetricSnapshot) {
        self.metrics.push(metric);
    }

    /// Add a breakpoint.
    pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.breakpoints.push(breakpoint);
    }
}

/// A log entry from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
    pub metadata: HashMap<String, String>,
}

/// Log level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// A metric snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub timestamp: u64,
    pub name: String,
    pub value: serde_json::Value,
    pub labels: HashMap<String, String>,
}

/// A breakpoint definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: Uuid,
    pub agent_id: crate::common::types::AgentId,
    pub condition: String,
    pub hit_count: u64,
    pub enabled: bool,
}

/// Debug command sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugCommand {
    /// Pause execution.
    Pause,
    /// Resume execution.
    Resume,
    /// Step to next operation.
    Step,
    /// Inspect state (key‑value).
    Inspect { key: String },
    /// Modify state (key‑value).
    Modify { key: String, value: serde_json::Value },
    /// Execute a custom script.
    Execute { script: String },
    /// List breakpoints.
    ListBreakpoints,
    /// Add a breakpoint.
    AddBreakpoint { condition: String },
    /// Remove a breakpoint.
    RemoveBreakpoint { id: Uuid },
    /// Collect metrics.
    CollectMetrics,
    /// Collect logs.
    CollectLogs,
    /// Terminate the agent (dangerous).
    Terminate,
}

/// Response from a debug command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugResponse {
    pub command_id: Uuid,
    pub agent_id: crate::common::types::AgentId,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub timestamp: u64,
}