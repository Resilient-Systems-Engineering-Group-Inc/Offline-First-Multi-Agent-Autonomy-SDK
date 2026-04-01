//! Debugger manager for coordinating distributed debugging.

use crate::error::{DebuggerError, Result};
use crate::session::{DebugSession, DebugCommand, DebugResponse, LogEntry, MetricSnapshot};
use dashmap::DashMap;
use mesh_transport::Transport;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Central debugger manager.
pub struct DebuggerManager<T: Transport + Send + Sync> {
    sessions: DashMap<Uuid, DebugSession>,
    transport: Arc<T>,
    agent_sessions: DashMap<crate::common::types::AgentId, Vec<Uuid>>,
}

impl<T: Transport + Send + Sync> DebuggerManager<T> {
    /// Create a new debugger manager.
    pub fn new(transport: T) -> Self {
        Self {
            sessions: DashMap::new(),
            transport: Arc::new(transport),
            agent_sessions: DashMap::new(),
        }
    }

    /// Start a new debug session for given agents.
    pub fn start_session(&self, agent_ids: Vec<crate::common::types::AgentId>) -> Uuid {
        let session = DebugSession::new(agent_ids.clone());
        let id = session.id;
        self.sessions.insert(id, session);
        for agent_id in agent_ids {
            self.agent_sessions
                .entry(agent_id)
                .or_insert_with(Vec::new)
                .push(id);
        }
        id
    }

    /// End a session.
    pub fn end_session(&self, session_id: Uuid) -> Result<()> {
        if let Some(mut session) = self.sessions.get_mut(&session_id) {
            session.end();
            // Remove from agent_sessions (optional)
        } else {
            return Err(DebuggerError::SessionNotFound(session_id.to_string()));
        }
        Ok(())
    }

    /// Send a debug command to an agent.
    pub async fn send_command(
        &self,
        session_id: Uuid,
        agent_id: crate::common::types::AgentId,
        command: DebugCommand,
    ) -> Result<DebugResponse> {
        // Verify session exists and includes the agent.
        if let Some(session) = self.sessions.get(&session_id) {
            if !session.agent_ids.contains(&agent_id) {
                return Err(DebuggerError::AgentNotFound(agent_id.to_string()));
            }
        } else {
            return Err(DebuggerError::SessionNotFound(session_id.to_string()));
        }

        let command_id = Uuid::new_v4();
        let message = serde_json::json!({
            "type": "debug_command",
            "command_id": command_id,
            "session_id": session_id,
            "command": command,
        });
        let payload = serde_json::to_vec(&message).map_err(DebuggerError::Serialization)?;
        self.transport
            .send_to(agent_id, payload)
            .await
            .map_err(|e| DebuggerError::Network(e.to_string()))?;

        // In a real implementation, we would wait for a response via events.
        // For now, return a dummy response.
        let response = DebugResponse {
            command_id,
            agent_id,
            success: true,
            data: None,
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        Ok(response)
    }

    /// Broadcast a command to all agents in a session.
    pub async fn broadcast_command(
        &self,
        session_id: Uuid,
        command: DebugCommand,
    ) -> Result<Vec<DebugResponse>> {
        let agents = if let Some(session) = self.sessions.get(&session_id) {
            session.agent_ids.clone()
        } else {
            return Err(DebuggerError::SessionNotFound(session_id.to_string()));
        };
        let mut responses = Vec::new();
        for agent_id in agents {
            let resp = self.send_command(session_id, agent_id, command.clone()).await?;
            responses.push(resp);
        }
        Ok(responses)
    }

    /// Handle an incoming debug event from an agent.
    pub async fn handle_event(
        &self,
        agent_id: crate::common::types::AgentId,
        payload: Vec<u8>,
    ) -> Result<()> {
        let event: serde_json::Value =
            serde_json::from_slice(&payload).map_err(DebuggerError::Serialization)?;
        // Process event (log, metric, breakpoint hit, etc.)
        // For simplicity, we just log.
        log::info!("Debug event from agent {}: {:?}", agent_id, event);
        Ok(())
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: Uuid) -> Option<DebugSession> {
        self.sessions.get(&session_id).map(|s| s.clone())
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<DebugSession> {
        self.sessions.iter().map(|s| s.clone()).collect()
    }

    /// Add a log entry to a session.
    pub fn add_log(&self, session_id: Uuid, entry: LogEntry) -> Result<()> {
        if let Some(mut session) = self.sessions.get_mut(&session_id) {
            session.add_log(entry);
            Ok(())
        } else {
            Err(DebuggerError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Add a metric snapshot to a session.
    pub fn add_metric(&self, session_id: Uuid, metric: MetricSnapshot) -> Result<()> {
        if let Some(mut session) = self.sessions.get_mut(&session_id) {
            session.add_metric(metric);
            Ok(())
        } else {
            Err(DebuggerError::SessionNotFound(session_id.to_string()))
        }
    }
}

/// Web‑based debugger UI server.
pub struct DebuggerWebServer {
    port: u16,
}

impl DebuggerWebServer {
    /// Create a new web server.
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Start the web server (requires warp).
    pub async fn start<T: Transport + Send + Sync + 'static>(
        self,
        manager: Arc<DebuggerManager<T>>,
    ) -> Result<()> {
        // This is a placeholder; actual implementation would define routes.
        log::info!("Debugger web server starting on port {}", self.port);
        Ok(())
    }
}