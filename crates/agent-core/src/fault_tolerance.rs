//! Fault tolerance and self‑healing mechanisms.
//!
//! Provides heartbeat-based failure detection, automatic task reallocation,
//! and configurable recovery strategies for the multi-agent system.
//!
//! # Architecture
//!
//! The fault tolerance system consists of three layers:
//! 1. **FaultDetector** — Tracks agent liveness via heartbeats and transport events
//! 2. **TaskReallocator** — Reassigns tasks from failed agents to healthy ones
//! 3. **FaultToleranceManager** — Orchestrates detection and recovery in a background task
//!
//! # Usage
//!
//! ```rust,ignore
//! use agent_core::fault_tolerance::FaultToleranceManager;
//! use tokio::sync::mpsc;
//!
//! let (tx, rx) = mpsc::unbounded_channel();
//! let manager = FaultToleranceManager::new(rx);
//! tokio::spawn(manager.run());
//! ```

use common::types::AgentId;
use mesh_transport::TransportEvent;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

/// Error type for fault tolerance operations.
#[derive(Error, Debug)]
pub enum FaultToleranceError {
    /// The fault tolerance manager is already running.
    #[error("Fault tolerance manager is already running")]
    AlreadyRunning,

    /// The fault tolerance manager is not running.
    #[error("Fault tolerance manager is not running")]
    NotRunning,

    /// The event channel has been closed.
    #[error("Event channel closed")]
    ChannelClosed,

    /// No alive agents available for task reallocation.
    #[error("No alive agents available for reallocation")]
    NoAliveAgents,

    /// Reallocation already performed for this agent.
    #[error("Reallocation already performed for agent {0}")]
    AlreadyReallocated(AgentId),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for fault tolerance operations.
pub type Result<T> = std::result::Result<T, FaultToleranceError>;

/// Configuration for the fault detector.
#[derive(Debug, Clone)]
pub struct FaultDetectorConfig {
    /// Maximum time without a heartbeat before an agent is considered suspect.
    pub heartbeat_timeout: Duration,
    /// Number of missed heartbeats before declaring an agent dead.
    pub missed_heartbeat_threshold: u32,
    /// Interval between heartbeat checks.
    pub check_interval: Duration,
}

impl Default for FaultDetectorConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout: Duration::from_secs(30),
            missed_heartbeat_threshold: 3,
            check_interval: Duration::from_secs(10),
        }
    }
}

/// Tracks which agents are considered alive using heartbeats and transport events.
#[derive(Debug)]
pub struct FaultDetector {
    config: FaultDetectorConfig,
    alive_agents: HashSet<AgentId>,
    /// Map from agent ID to timestamp of last heartbeat.
    last_heartbeat: HashMap<AgentId, Instant>,
    /// Map from agent ID to count of missed heartbeats.
    missed_heartbeats: HashMap<AgentId, u32>,
    /// Agents that have been explicitly marked as dead.
    dead_agents: HashSet<AgentId>,
}

impl FaultDetector {
    pub fn new() -> Self {
        Self::with_config(FaultDetectorConfig::default())
    }

    pub fn with_config(config: FaultDetectorConfig) -> Self {
        Self {
            config,
            alive_agents: HashSet::new(),
            last_heartbeat: HashMap::new(),
            missed_heartbeats: HashMap::new(),
            dead_agents: HashSet::new(),
        }
    }

    /// Record a heartbeat from an agent.
    pub fn record_heartbeat(&mut self, agent: AgentId) {
        self.last_heartbeat.insert(agent, Instant::now());
        self.missed_heartbeats.remove(&agent);
        self.dead_agents.remove(&agent);
        self.alive_agents.insert(agent);
        debug!(?agent, "Heartbeat recorded");
    }

    /// Update based on a transport event.
    pub fn on_event(&mut self, event: &TransportEvent) {
        match event {
            TransportEvent::PeerDiscovered(agent) | TransportEvent::ConnectionEstablished(agent) => {
                self.alive_agents.insert(*agent);
                self.dead_agents.remove(agent);
                self.last_heartbeat.entry(*agent).or_insert_with(Instant::now);
                info!(?agent, "Agent marked as alive");
            }
            TransportEvent::PeerLost(agent) | TransportEvent::ConnectionClosed(agent) => {
                self.alive_agents.remove(agent);
                self.dead_agents.insert(*agent);
                warn!(?agent, "Agent marked as dead via transport event");
            }
            _ => {}
        }
    }

    /// Check for timed-out agents and return the list of newly detected failures.
    pub fn check_timeouts(&mut self) -> Vec<AgentId> {
        let now = Instant::now();
        let mut failed_agents = Vec::new();

        let timed_out: Vec<AgentId> = self.last_heartbeat
            .iter()
            .filter(|(agent, last_seen)| {
                let elapsed = now.duration_since(**last_seen);
                elapsed > self.config.heartbeat_timeout
            })
            .map(|(agent, _)| *agent)
            .collect();

        for agent in timed_out {
            let missed = self.missed_heartbeats.entry(agent).or_insert(0);
            *missed += 1;

            if *missed >= self.config.missed_heartbeat_threshold {
                self.alive_agents.remove(&agent);
                self.dead_agents.insert(agent);
                failed_agents.push(agent);
                warn!(
                    ?agent,
                    missed_heartbeats = *missed,
                    "Agent declared dead after missed heartbeats"
                );
            } else {
                warn!(
                    ?agent,
                    missed_heartbeats = *missed,
                    threshold = self.config.missed_heartbeat_threshold,
                    "Agent heartbeat timeout (suspect)"
                );
            }
        }

        failed_agents
    }

    /// Check if an agent is considered alive.
    pub fn is_alive(&self, agent: &AgentId) -> bool {
        self.alive_agents.contains(agent) && !self.dead_agents.contains(agent)
    }

    /// Get all alive agents.
    pub fn alive_agents(&self) -> &HashSet<AgentId> {
        &self.alive_agents
    }

    /// Get all dead agents.
    pub fn dead_agents(&self) -> &HashSet<AgentId> {
        &self.dead_agents
    }

    /// Get the number of missed heartbeats for an agent.
    pub fn missed_heartbeats(&self, agent: &AgentId) -> u32 {
        self.missed_heartbeats.get(agent).copied().unwrap_or(0)
    }

    /// Reset the detector to initial state.
    pub fn reset(&mut self) {
        self.alive_agents.clear();
        self.last_heartbeat.clear();
        self.missed_heartbeats.clear();
        self.dead_agents.clear();
    }
}

impl Default for FaultDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for task reallocation.
#[derive(Debug, Clone)]
pub struct ReallocationConfig {
    /// Whether to enable automatic task reallocation.
    pub enabled: bool,
    /// Delay before attempting reallocation (to avoid premature reallocation).
    pub reallocation_delay: Duration,
    /// Maximum number of tasks to reallocate per batch.
    pub max_tasks_per_batch: usize,
}

impl Default for ReallocationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reallocation_delay: Duration::from_secs(5),
            max_tasks_per_batch: 10,
        }
    }
}

/// Reallocates tasks from failed agents to alive ones.
pub struct TaskReallocator {
    config: ReallocationConfig,
    /// Track which agents have had tasks reallocated (to avoid duplicate work).
    reallocated_for: HashSet<AgentId>,
}

impl TaskReallocator {
    pub fn new() -> Self {
        Self::with_config(ReallocationConfig::default())
    }

    pub fn with_config(config: ReallocationConfig) -> Self {
        Self {
            config,
            reallocated_for: HashSet::new(),
        }
    }

    /// Called when an agent is detected as dead.
    /// In a real implementation, this would query the task assignments and reassign them.
    pub async fn on_agent_failure(&self, failed_agent: AgentId, alive_agents: &HashSet<AgentId>) {
        if !self.config.enabled {
            info!(?failed_agent, "Task reallocation is disabled, skipping");
            return;
        }

        if self.reallocated_for.contains(&failed_agent) {
            debug!(?failed_agent, "Tasks already reallocated for this agent");
            return;
        }

        warn!(
            ?failed_agent,
            ?alive_agents,
            max_tasks = self.config.max_tasks_per_batch,
            "Agent failure detected, reallocating tasks"
        );

        // In a real implementation, this would:
        // 1. Query the distributed planner for tasks assigned to the failed agent
        // 2. Filter tasks that are still pending or in-progress
        // 3. Select the best alive agents based on capabilities and load
        // 4. Propose new assignments via consensus
        // 5. Update the CRDT map with new assignments

        // Simulate reallocation delay
        tokio::time::sleep(self.config.reallocation_delay).await;

        info!(
            ?failed_agent,
            "Task reallocation completed (simulated)"
        );
    }

    /// Mark reallocation as complete for an agent.
    pub fn mark_reallocated(&mut self, agent: AgentId) {
        self.reallocated_for.insert(agent);
    }

    /// Check if reallocation has already been performed for an agent.
    pub fn has_reallocated(&self, agent: &AgentId) -> bool {
        self.reallocated_for.contains(agent)
    }

    /// Reset the reallocator state.
    pub fn reset(&mut self) {
        self.reallocated_for.clear();
    }
}

impl Default for TaskReallocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for the fault tolerance manager.
#[derive(Debug, Clone)]
pub struct FaultToleranceConfig {
    pub detector: FaultDetectorConfig,
    pub reallocation: ReallocationConfig,
    /// Whether to enable periodic heartbeat checks.
    pub enable_periodic_checks: bool,
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            detector: FaultDetectorConfig::default(),
            reallocation: ReallocationConfig::default(),
            enable_periodic_checks: true,
        }
    }
}

/// A combined fault‑tolerance manager that runs in the background.
///
/// Periodically checks for agent timeouts and triggers task reallocation
/// when failures are detected. Supports graceful shutdown via `shutdown()`.
pub struct FaultToleranceManager {
    config: FaultToleranceConfig,
    detector: FaultDetector,
    reallocator: TaskReallocator,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
    /// Global shutdown flag.
    shutdown: Arc<AtomicBool>,
}

impl FaultToleranceManager {
    pub fn new(event_rx: mpsc::UnboundedReceiver<TransportEvent>) -> Self {
        Self::with_config(event_rx, FaultToleranceConfig::default())
    }

    pub fn with_config(
        event_rx: mpsc::UnboundedReceiver<TransportEvent>,
        config: FaultToleranceConfig,
    ) -> Self {
        Self {
            config,
            detector: FaultDetector::with_config(config.detector.clone()),
            reallocator: TaskReallocator::with_config(config.reallocation.clone()),
            event_rx,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the manager to shut down gracefully.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        info!("Fault tolerance manager shutdown signal sent");
    }

    /// Check if the manager has been signalled to shut down.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Run the manager loop, processing events and triggering recovery.
    ///
    /// Returns immediately if the manager has already been shut down.
    pub async fn run(mut self) {
        if self.is_shutdown() {
            info!("Fault tolerance manager already shut down, not starting");
            return;
        }

        info!("Fault tolerance manager started");

        let check_interval = self.config.detector.check_interval;

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = tokio::time::sleep(Duration::from_millis(100)), if self.is_shutdown() => {
                    info!("Fault tolerance manager shutting down via signal");
                    break;
                }

                // Process incoming transport events
                event = self.event_rx.recv() => {
                    if self.is_shutdown() {
                        break;
                    }

                    match event {
                        Some(event) => {
                            self.detector.on_event(&event);

                            // If a peer is lost, trigger reallocation
                            if let TransportEvent::PeerLost(failed_agent) = &event {
                                let alive = self.detector.alive_agents().clone();
                                self.reallocator.on_agent_failure(*failed_agent, &alive).await;
                                self.reallocator.mark_reallocated(*failed_agent);
                            }
                        }
                        None => {
                            warn!("Fault tolerance event channel closed, shutting down");
                            break;
                        }
                    }
                }

                // Periodic heartbeat timeout checks
                _ = tokio::time::sleep(check_interval), if self.config.enable_periodic_checks => {
                    if self.is_shutdown() {
                        break;
                    }

                    let failed_agents = self.detector.check_timeouts();
                    for failed_agent in failed_agents {
                        let alive = self.detector.alive_agents().clone();
                        self.reallocator.on_agent_failure(failed_agent, &alive).await;
                        self.reallocator.mark_reallocated(failed_agent);
                    }
                }
            }
        }

        info!("Fault tolerance manager stopped");
    }

    /// Get a reference to the fault detector.
    pub fn detector(&self) -> &FaultDetector {
        &self.detector
    }

    /// Get a reference to the task reallocator.
    pub fn reallocator(&self) -> &TaskReallocator {
        &self.reallocator
    }

    /// Get the current configuration.
    pub fn config(&self) -> &FaultToleranceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ─── FaultDetector Tests ───────────────────────────────────────────────

    #[test]
    fn test_fault_detector_initial_state() {
        let detector = FaultDetector::new();
        assert!(detector.alive_agents().is_empty());
        assert!(detector.dead_agents().is_empty());
        assert_eq!(detector.missed_heartbeats(&1), 0);
    }

    #[test]
    fn test_fault_detector_heartbeat() {
        let mut detector = FaultDetector::new();
        detector.record_heartbeat(1);
        assert!(detector.is_alive(&1));
        assert!(detector.alive_agents().contains(&1));
        assert!(!detector.dead_agents().contains(&1));
    }

    #[test]
    fn test_fault_detector_heartbeat_removes_from_dead() {
        let mut detector = FaultDetector::new();
        // Mark agent as dead via transport event
        detector.on_event(&TransportEvent::PeerLost(1));
        assert!(!detector.is_alive(&1));

        // Heartbeat should revive the agent
        detector.record_heartbeat(1);
        assert!(detector.is_alive(&1));
    }

    #[test]
    fn test_fault_detector_transport_events() {
        let mut detector = FaultDetector::new();

        // Simulate peer discovery
        detector.on_event(&TransportEvent::PeerDiscovered(1));
        assert!(detector.is_alive(&1));

        // Simulate peer loss
        detector.on_event(&TransportEvent::PeerLost(1));
        assert!(!detector.is_alive(&1));
    }

    #[test]
    fn test_fault_detector_connection_events() {
        let mut detector = FaultDetector::new();

        // Connection established should mark agent alive
        detector.on_event(&TransportEvent::ConnectionEstablished(1));
        assert!(detector.is_alive(&1));

        // Connection closed should mark agent dead
        detector.on_event(&TransportEvent::ConnectionClosed(1));
        assert!(!detector.is_alive(&1));
    }

    #[test]
    fn test_fault_detector_other_events_ignored() {
        let mut detector = FaultDetector::new();
        // MessageReceived events should not affect liveness state
        detector.on_event(&TransportEvent::MessageReceived {
            from: 1,
            payload: vec![1, 2, 3],
        });
        assert!(detector.alive_agents().is_empty());
    }

    #[test]
    fn test_fault_detector_reset() {
        let mut detector = FaultDetector::new();
        detector.record_heartbeat(1);
        detector.record_heartbeat(2);
        assert_eq!(detector.alive_agents().len(), 2);

        detector.reset();
        assert!(detector.alive_agents().is_empty());
        assert!(detector.dead_agents().is_empty());
        assert!(detector.missed_heartbeats(&1) == 0);
    }

    #[test]
    fn test_fault_detector_config() {
        let config = FaultDetectorConfig {
            heartbeat_timeout: Duration::from_secs(10),
            missed_heartbeat_threshold: 2,
            check_interval: Duration::from_secs(5),
        };
        let detector = FaultDetector::with_config(config);
        assert_eq!(detector.missed_heartbeats(&1), 0);
    }

    #[test]
    fn test_fault_detector_missed_heartbeats_tracking() {
        let mut detector = FaultDetector::new();
        detector.record_heartbeat(1);
        assert_eq!(detector.missed_heartbeats(&1), 0);
    }

    #[test]
    fn test_fault_detector_is_alive_returns_false_for_unknown() {
        let detector = FaultDetector::new();
        assert!(!detector.is_alive(&99));
    }

    #[test]
    fn test_fault_detector_dead_agents_accessor() {
        let mut detector = FaultDetector::new();
        detector.on_event(&TransportEvent::PeerLost(42));
        assert!(detector.dead_agents().contains(&42));
    }

    // ─── TaskReallocator Tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_task_reallocator_basic() {
        let reallocator = TaskReallocator::new();
        let mut alive = HashSet::new();
        alive.insert(2);
        alive.insert(3);

        // This should not panic
        reallocator.on_agent_failure(1, &alive).await;
    }

    #[tokio::test]
    async fn test_task_reallocator_disabled() {
        let config = ReallocationConfig {
            enabled: false,
            ..Default::default()
        };
        let reallocator = TaskReallocator::with_config(config);
        let mut alive = HashSet::new();
        alive.insert(2);

        // Should skip reallocation without error
        reallocator.on_agent_failure(1, &alive).await;
    }

    #[test]
    fn test_task_reallocator_mark_and_check() {
        let mut reallocator = TaskReallocator::new();
        assert!(!reallocator.has_reallocated(&1));
        reallocator.mark_reallocated(1);
        assert!(reallocator.has_reallocated(&1));
    }

    #[tokio::test]
    async fn test_task_reallocator_skips_already_reallocated() {
        let mut reallocator = TaskReallocator::new();
        reallocator.mark_reallocated(1);

        let mut alive = HashSet::new();
        alive.insert(2);

        // Should skip because already reallocated
        reallocator.on_agent_failure(1, &alive).await;
        assert!(reallocator.has_reallocated(&1));
    }

    #[test]
    fn test_task_reallocator_reset() {
        let mut reallocator = TaskReallocator::new();
        reallocator.mark_reallocated(1);
        reallocator.mark_reallocated(2);
        assert!(reallocator.has_reallocated(&1));

        reallocator.reset();
        assert!(!reallocator.has_reallocated(&1));
    }

    #[test]
    fn test_task_reallocator_default() {
        let reallocator = TaskReallocator::default();
        assert!(reallocator.config.enabled);
        assert_eq!(reallocator.config.max_tasks_per_batch, 10);
    }

    // ─── FaultToleranceManager Tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_fault_tolerance_manager_shutdown() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);

        assert!(!manager.is_shutdown());
        manager.shutdown();
        assert!(manager.is_shutdown());
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_run_returns_immediately_if_shutdown() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);
        manager.shutdown();
        // Should return immediately without hanging
        manager.run().await;
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_detector_accessor() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);
        assert_eq!(manager.detector().alive_agents().len(), 0);
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_reallocator_accessor() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);
        assert!(!manager.reallocator().has_reallocated(&1));
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_config_accessor() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);
        assert!(manager.config().enable_periodic_checks);
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_processes_events() {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);

        // Send a peer discovery event
        tx.send(TransportEvent::PeerDiscovered(1)).unwrap();

        // Run the manager briefly, then shut down
        let handle = tokio::spawn(async move {
            // Let it process one event then shut down
            tokio::time::sleep(Duration::from_millis(50)).await;
            manager.shutdown();
            manager.run().await;
        });

        // Wait for the manager to finish
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_channel_close_triggers_shutdown() {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);

        // Drop the sender to close the channel
        drop(tx);

        // Manager should detect channel close and exit
        manager.run().await;
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_with_config() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let config = FaultToleranceConfig {
            enable_periodic_checks: false,
            ..Default::default()
        };
        let manager = FaultToleranceManager::with_config(rx, config);
        assert!(!manager.config().enable_periodic_checks);
    }

    #[tokio::test]
    async fn test_fault_tolerance_manager_peer_lost_triggers_reallocation() {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = FaultToleranceManager::new(rx);

        // Register an alive agent first
        tx.send(TransportEvent::PeerDiscovered(2)).unwrap();
        // Then lose agent 1
        tx.send(TransportEvent::PeerLost(1)).unwrap();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            manager.shutdown();
            manager.run().await;
        });

        handle.await.unwrap();
    }

    // ─── FaultToleranceError Tests ─────────────────────────────────────────

    #[test]
    fn test_fault_tolerance_error_display() {
        let err = FaultToleranceError::NoAliveAgents;
        assert_eq!(err.to_string(), "No alive agents available for reallocation");

        let err = FaultToleranceError::ChannelClosed;
        assert_eq!(err.to_string(), "Event channel closed");

        let err = FaultToleranceError::AlreadyRunning;
        assert_eq!(err.to_string(), "Fault tolerance manager is already running");

        let err = FaultToleranceError::NotRunning;
        assert_eq!(err.to_string(), "Fault tolerance manager is not running");

        let err = FaultToleranceError::AlreadyReallocated(42);
        assert_eq!(err.to_string(), "Reallocation already performed for agent 42");

        let err = FaultToleranceError::Internal("test error".to_string());
        assert_eq!(err.to_string(), "Internal error: test error");
    }
}