//! Schema definitions for the overall SDK configuration.

use serde::{Deserialize, Serialize};

/// Root configuration for the Offline‑First Multi‑Agent Autonomy SDK.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Configuration {
    /// Mesh transport configuration.
    pub mesh: MeshConfig,
    /// Agent core configuration.
    pub agent: AgentConfig,
    /// State synchronization configuration.
    pub state_sync: StateSyncConfig,
    /// Resource monitoring configuration.
    pub resource_monitor: ResourceMonitorConfig,
    /// Planning configuration.
    pub planning: PlanningConfig,
    /// Security configuration.
    pub security: SecurityConfig,
    /// Logging configuration.
    pub logging: LoggingConfig,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            mesh: MeshConfig::default(),
            agent: AgentConfig::default(),
            state_sync: StateSyncConfig::default(),
            resource_monitor: ResourceMonitorConfig::default(),
            planning: PlanningConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Mesh transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MeshConfig {
    /// Backend to use ("libp2p", "webrtc", "lora", "in_memory").
    pub backend: String,
    /// Listening address (e.g., "0.0.0.0:5000").
    pub listen_addr: String,
    /// Bootstrap peers (list of multiaddresses).
    pub bootstrap_peers: Vec<String>,
    /// Enable encryption.
    pub enable_encryption: bool,
    /// Enable discovery.
    pub enable_discovery: bool,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            backend: "libp2p".to_string(),
            listen_addr: "0.0.0.0:5000".to_string(),
            bootstrap_peers: vec![],
            enable_encryption: true,
            enable_discovery: true,
        }
    }
}

/// Agent core configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Agent ID (auto‑generated if empty).
    pub agent_id: String,
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Heartbeat interval in seconds.
    pub heartbeat_interval_secs: u64,
    /// Enable fault tolerance.
    pub fault_tolerance: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_id: "".to_string(),
            max_concurrent_tasks: 10,
            heartbeat_interval_secs: 5,
            fault_tolerance: true,
        }
    }
}

/// State synchronization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StateSyncConfig {
    /// CRDT map synchronization interval in seconds.
    pub sync_interval_secs: u64,
    /// Enable delta compression.
    pub delta_compression: bool,
    /// Enable persistence.
    pub persistence: bool,
}

impl Default for StateSyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_secs: 2,
            delta_compression: true,
            persistence: true,
        }
    }
}

/// Resource monitoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceMonitorConfig {
    /// Collection interval in seconds.
    pub collection_interval_secs: u64,
    /// Alert thresholds (CPU %, memory %, etc.).
    pub thresholds: Thresholds,
}

impl Default for ResourceMonitorConfig {
    fn default() -> Self {
        Self {
            collection_interval_secs: 10,
            thresholds: Thresholds::default(),
        }
    }
}

/// Resource thresholds for alerting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    /// CPU usage percentage.
    pub cpu_percent: f64,
    /// Memory usage percentage.
    pub memory_percent: f64,
    /// Disk usage percentage.
    pub disk_percent: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cpu_percent: 80.0,
            memory_percent: 85.0,
            disk_percent: 90.0,
        }
    }
}

/// Planning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningConfig {
    /// Planner type ("local", "distributed", "rl").
    pub planner_type: String,
    /// Enable deadline‑aware scheduling.
    pub deadline_aware: bool,
    /// Enable dependency‑aware scheduling.
    pub dependency_aware: bool,
}

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            planner_type: "distributed".to_string(),
            deadline_aware: true,
            dependency_aware: true,
        }
    }
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable authentication.
    pub enable_authentication: bool,
    /// Enable encryption.
    pub enable_encryption: bool,
    /// Shared secret for simple auth (not for production).
    pub shared_secret: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_authentication: false,
            enable_encryption: true,
            shared_secret: "".to_string(),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level ("trace", "debug", "info", "warn", "error").
    pub level: String,
    /// Enable JSON output.
    pub json: bool,
    /// Enable file output.
    pub file: Option<String>,
    /// Enable Prometheus metrics.
    pub prometheus: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
            file: None,
            prometheus: true,
        }
    }
}