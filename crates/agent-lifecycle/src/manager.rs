//! Lifecycle manager for agents.

use crate::error::{LifecycleError, Result};
use crate::health::{HealthMonitor, HealthStatus};
use crate::state::{AgentState, StateMachine};
use common::types::AgentId;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use resource_monitor::SysinfoMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Configuration for the lifecycle manager.
#[derive(Debug, Clone)]
pub struct LifecycleManagerConfig {
    /// Agent ID.
    pub agent_id: AgentId,
    /// Transport configuration.
    pub transport_config: MeshTransportConfig,
    /// Enable health monitoring.
    pub enable_health_monitoring: bool,
    /// Health check interval in seconds.
    pub health_check_interval_secs: u64,
    /// Graceful shutdown timeout in seconds.
    pub shutdown_timeout_secs: u64,
    /// Enable automatic recovery from failed state.
    pub enable_auto_recovery: bool,
    /// Maximum recovery attempts.
    pub max_recovery_attempts: usize,
}

impl Default for LifecycleManagerConfig {
    fn default() -> Self {
        Self {
            agent_id: 1,
            transport_config: MeshTransportConfig::in_memory(),
            enable_health_monitoring: true,
            health_check_interval_secs: 30,
            shutdown_timeout_secs: 30,
            enable_auto_recovery: true,
            max_recovery_attempts: 3,
        }
    }
}

/// Information about a managed agent.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent ID.
    pub id: AgentId,
    /// Current state.
    pub state: AgentState,
    /// Health status.
    pub health: HealthStatus,
    /// Time when the agent was started.
    pub started_at: Option<std::time::SystemTime>,
    /// Time when the agent last changed state.
    pub last_state_change: std::time::SystemTime,
    /// Recovery attempts count.
    pub recovery_attempts: usize,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

/// Lifecycle manager for a single agent.
pub struct LifecycleManager {
    config: LifecycleManagerConfig,
    state_machine: Mutex<StateMachine>,
    health_monitor: Mutex<Option<HealthMonitor>>,
    transport: Mutex<Option<MeshTransport>>,
    agent_info: RwLock<AgentInfo>,
    recovery_attempts: Mutex<usize>,
    shutdown_signal: tokio::sync::watch::Sender<bool>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager.
    pub fn new(config: LifecycleManagerConfig) -> Result<Self> {
        let agent_info = AgentInfo {
            id: config.agent_id,
            state: AgentState::Initializing,
            health: HealthStatus::Unknown,
            started_at: None,
            last_state_change: std::time::SystemTime::now(),
            recovery_attempts: 0,
            metadata: HashMap::new(),
        };

        let (shutdown_signal, _) = tokio::sync::watch::channel(false);

        Ok(Self {
            config,
            state_machine: Mutex::new(StateMachine::new()),
            health_monitor: Mutex::new(None),
            transport: Mutex::new(None),
            agent_info: RwLock::new(agent_info),
            recovery_attempts: Mutex::new(0),
            shutdown_signal,
        })
    }

    /// Initialize the agent (first step in lifecycle).
    pub async fn initialize(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        // Transition from Initializing to Ready
        state_machine
            .transition_to(AgentState::Ready)
            .map_err(|e| LifecycleError::Internal(e))?;

        // Update agent info
        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Ready;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} initialized and ready", self.config.agent_id);
        Ok(())
    }

    /// Start the agent.
    pub async fn start(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        // Transition to Starting
        state_machine
            .transition_to(AgentState::Starting)
            .map_err(|e| LifecycleError::Internal(e))?;

        // Update agent info
        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Starting;
        agent_info.started_at = Some(std::time::SystemTime::now());
        agent_info.last_state_change = std::time::SystemTime::now();

        // Initialize transport
        let transport = MeshTransport::new(self.config.transport_config.clone())
            .await
            .map_err(LifecycleError::TransportError)?;
        
        let mut transport_guard = self.transport.lock().await;
        *transport_guard = Some(transport);

        // Initialize health monitor if enabled
        if self.config.enable_health_monitoring {
            let mut monitor = HealthMonitor::new();
            
            // Add resource monitor
            let resource_monitor = SysinfoMonitor::new();
            monitor = monitor.with_resource_monitor(resource_monitor);
            
            let mut health_monitor_guard = self.health_monitor.lock().await;
            *health_monitor_guard = Some(monitor);
        }

        // Transition to Running
        state_machine
            .transition_to(AgentState::Running)
            .map_err(|e| LifecycleError::Internal(e))?;

        agent_info.state = AgentState::Running;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} started and running", self.config.agent_id);
        
        // Start background tasks
        self.start_background_tasks().await?;

        Ok(())
    }

    /// Stop the agent gracefully.
    pub async fn stop(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        // Transition to Stopping
        state_machine
            .transition_to(AgentState::Stopping)
            .map_err(|e| LifecycleError::Internal(e))?;

        // Update agent info
        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Stopping;
        agent_info.last_state_change = std::time::SystemTime::now();

        // Send shutdown signal to background tasks
        self.shutdown_signal.send(true).map_err(|_| {
            LifecycleError::Internal("Failed to send shutdown signal".to_string())
        })?;

        // Stop transport
        let mut transport_guard = self.transport.lock().await;
        if let Some(transport) = transport_guard.as_mut() {
            transport.stop().await.map_err(LifecycleError::TransportError)?;
        }
        *transport_guard = None;

        // Transition to Stopped
        state_machine
            .transition_to(AgentState::Stopped)
            .map_err(|e| LifecycleError::Internal(e))?;

        agent_info.state = AgentState::Stopped;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} stopped gracefully", self.config.agent_id);
        Ok(())
    }

    /// Suspend the agent (pause operations).
    pub async fn suspend(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        if !state_machine.can_transition_to(AgentState::Suspended) {
            return Err(LifecycleError::InvalidTransition(
                state_machine.current_state().to_string(),
                AgentState::Suspended.to_string(),
            ));
        }

        state_machine
            .transition_to(AgentState::Suspended)
            .map_err(|e| LifecycleError::Internal(e))?;

        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Suspended;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} suspended", self.config.agent_id);
        Ok(())
    }

    /// Resume the agent from suspended state.
    pub async fn resume(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        if !state_machine.can_transition_to(AgentState::Running) {
            return Err(LifecycleError::InvalidTransition(
                state_machine.current_state().to_string(),
                AgentState::Running.to_string(),
            ));
        }

        state_machine
            .transition_to(AgentState::Running)
            .map_err(|e| LifecycleError::Internal(e))?;

        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Running;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} resumed", self.config.agent_id);
        Ok(())
    }

    /// Put agent in maintenance mode.
    pub async fn enter_maintenance(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        if !state_machine.can_transition_to(AgentState::Maintenance) {
            return Err(LifecycleError::InvalidTransition(
                state_machine.current_state().to_string(),
                AgentState::Maintenance.to_string(),
            ));
        }

        state_machine
            .transition_to(AgentState::Maintenance)
            .map_err(|e| LifecycleError::Internal(e))?;

        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Maintenance;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} entered maintenance mode", self.config.agent_id);
        Ok(())
    }

    /// Exit maintenance mode.
    pub async fn exit_maintenance(&self) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        
        if !state_machine.can_transition_to(AgentState::Ready) {
            return Err(LifecycleError::InvalidTransition(
                state_machine.current_state().to_string(),
                AgentState::Ready.to_string(),
            ));
        }

        state_machine
            .transition_to(AgentState::Ready)
            .map_err(|e| LifecycleError::Internal(e))?;

        let mut agent_info = self.agent_info.write().await;
        agent_info.state = AgentState::Ready;
        agent_info.last_state_change = std::time::SystemTime::now();

        info!("Agent {} exited maintenance mode", self.config.agent_id);
        Ok(())
    }

    /// Perform a health check.
    pub async fn check_health(&self) -> Result<HealthStatus> {
        let mut health_monitor_guard = self.health_monitor.lock().await;
        
        if let Some(ref mut monitor) = *health_monitor_guard {
            let result = monitor.check().await.map_err(|e| {
                LifecycleError::HealthCheckFailed(format!("Health check failed: {}", e))
            })?;
            
            // Update agent info
            let mut agent_info = self.agent_info.write().await;
            agent_info.health = result.status;
            
            // If unhealthy and auto-recovery is enabled, attempt recovery
            if result.status == HealthStatus::Unhealthy && self.config.enable_auto_recovery {
                self.attempt_recovery().await?;
            }
            
            Ok(result.status)
        } else {
            warn!("Health monitoring is disabled");
            Ok(HealthStatus::Unknown)
        }
    }

    /// Attempt to recover from failed state.
    async fn attempt_recovery(&self) -> Result<()> {
        let mut recovery_attempts_guard = self.recovery_attempts.lock().await;
        
        if *recovery_attempts_guard >= self.config.max_recovery_attempts {
            error!(
                "Max recovery attempts ({}) exceeded for agent {}",
                self.config.max_recovery_attempts, self.config.agent_id
            );
            return Err(LifecycleError::HealthCheckFailed(
                "Max recovery attempts exceeded".to_string(),
            ));
        }
        
        *recovery_attempts_guard += 1;
        
        let mut agent_info = self.agent_info.write().await;
        agent_info.recovery_attempts = *recovery_attempts_guard;
        
        info!(
            "Attempting recovery for agent {} (attempt {}/{})",
            self.config.agent_id,
            *recovery_attempts_guard,
            self.config.max_recovery_attempts
        );
        
        // Simple recovery strategy: stop and restart
        // In a real implementation, this would be more sophisticated
        drop(agent_info); // Release lock before async operations
        
        // Transition to Ready state
        let mut state_machine = self.state_machine.lock().await;
        if state_machine.can_transition_to(AgentState::Ready) {
            state_machine
                .transition_to(AgentState::Ready)
                .map_err(|e| LifecycleError::Internal(e))?;
            
            let mut agent_info = self.agent_info.write().await;
            agent_info.state = AgentState::Ready;
            agent_info.health = HealthStatus::Healthy;
            agent_info.last_state_change = std::time::SystemTime::now();
            
            info!("Agent {} recovered to Ready state", self.config.agent_id);
            Ok(())
        } else {
            Err(LifecycleError::HealthCheckFailed(
                "Cannot transition to Ready state for recovery".to_string(),
            ))
        }
    }

    /// Start background tasks (health monitoring, etc.).
    async fn start_background_tasks(&self) -> Result<()> {
        if self.config.enable_health_monitoring {
            let manager = Arc::new(self.clone_manager());
            let shutdown_rx = self.shutdown_signal.subscribe();
            
            tokio::spawn(async move {
                manager.run_health_monitoring(shutdown_rx).await;
            });
            
            info!("Started health monitoring background task");
        }
        
        Ok(())
    }

    /// Run health monitoring in a background task.
    async fn run_health_monitoring(&self, mut shutdown_rx: tokio::sync::watch::Receiver<bool>) {
        let interval = std::time::Duration::from_secs(self.config.health_check_interval_secs);
        
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    match self.check_health().await {
                        Ok(status) => {
                            debug!("Health check completed: {}", status);
                        }
                        Err(e) => {
                            error!("Health check error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        debug!("Health monitoring task shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Get current agent information.
    pub async fn get_agent_info(&self) -> AgentInfo {
        self.agent_info.read().await.clone()
    }

    /// Get current state.
    pub async fn get_state(&self) -> AgentState {
        let state_machine = self.state_machine.lock().await;
        state_machine.current_state()
    }

    /// Check if agent is operational.
    pub async fn is_operational(&self) -> bool {
        let state_machine = self.state_machine.lock().await;
        state_machine.is_operational()
    }

    /// Clone the manager for background tasks.
    fn clone_manager(&self) -> Self {
        Self {
            config: self.config.clone(),
            state_machine: Mutex::new(StateMachine::new()),
            health_monitor: Mutex::new(None),
            transport: Mutex::new(None),
            agent_info: RwLock::new(AgentInfo {
                id: self.config.agent_id,
                state: AgentState::Initializing,
                health: HealthStatus::Unknown,
                started_at: None,
                last_state_change: std::time::SystemTime::now(),
                recovery_attempts: 0,
                metadata: HashMap::new(),
            }),
            recovery_attempts: Mutex::new(0),
            shutdown_signal: self.shutdown_signal.clone(),
        }
    }
}

impl Clone for LifecycleManager {
    fn clone(&self) -> Self {
        self.clone_manager()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lifecycle_manager_creation() {
        let config = LifecycleManagerConfig::default();
        let manager = LifecycleManager::new(config).unwrap();
        
        let state = manager.get_state().await;
        assert_eq!(state, AgentState::Initializing);
        
        let info = manager.get_agent_info().await;
        assert_eq!(info.id, 1);
        assert_eq!(info.state, AgentState::Initializing);
    }

    #[tokio::test]
    async fn test_initialize() {
        let config = LifecycleManagerConfig::default();
        let manager = LifecycleManager::new(config).unwrap();
        
        manager.initialize().await.unwrap();
        
        let state = manager.get_state().await;
        assert_eq!(state, AgentState::Ready);
        
        let info = manager.get_agent_info().await;
        assert_eq!(info.state, AgentState::Ready);
    }

    #[tokio::test]
    async fn test_start_stop() {
        let config = LifecycleManagerConfig {
            enable_health_monitoring: false, // Disable for test
            ..Default::default()
        };
        let manager = LifecycleManager::new(config).unwrap();
        
        manager.initialize().await.unwrap();
        manager.start().await.unwrap();
        
        let state = manager.get_state().await;
        assert_eq!(state, AgentState::Running);
        
        manager.stop().await.unwrap();
        
        let state = manager.get_state().await;
        assert_eq!(state, AgentState::Stopped);
    }
}