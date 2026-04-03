//! Core types for container integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Container runtime type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeType {
    /// Docker runtime.
    Docker,
    /// Containerd runtime.
    Containerd,
    /// Podman runtime.
    Podman,
    /// Custom runtime.
    Custom(String),
}

/// Container specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Container name.
    pub name: String,
    /// Container image.
    pub image: String,
    /// Command to run (overrides ENTRYPOINT).
    pub command: Option<Vec<String>>,
    /// Command arguments (overrides CMD).
    pub args: Option<Vec<String>>,
    /// Environment variables.
    pub env: Vec<String>,
    /// Working directory.
    pub working_dir: Option<String>,
    /// User to run as (user:group).
    pub user: Option<String>,
    /// Whether to allocate a TTY.
    pub tty: bool,
    /// Whether to run in interactive mode.
    pub interactive: bool,
    /// Whether to remove container automatically when it stops.
    pub auto_remove: bool,
    /// Hostname.
    pub hostname: Option<String>,
    /// Domain name.
    pub domainname: Option<String>,
    /// Labels.
    pub labels: HashMap<String, String>,
    /// Annotations.
    pub annotations: HashMap<String, String>,
}

impl Default for ContainerSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            image: String::new(),
            command: None,
            args: None,
            env: Vec::new(),
            working_dir: None,
            user: None,
            tty: false,
            interactive: false,
            auto_remove: false,
            hostname: None,
            domainname: None,
            labels: HashMap::new(),
            annotations: HashMap::new(),
        }
    }
}

/// Resource constraints for containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// CPU shares (relative weight).
    pub cpu_shares: Option<u64>,
    /// CPU quota in microseconds.
    pub cpu_quota: Option<i64>,
    /// CPU period in microseconds.
    pub cpu_period: Option<u64>,
    /// CPUs to use (e.g., "1.5").
    pub cpus: Option<f64>,
    /// CPU set (e.g., "0-3").
    pub cpuset_cpus: Option<String>,
    /// Memory limit in bytes.
    pub memory: Option<u64>,
    /// Memory swap limit in bytes.
    pub memory_swap: Option<i64>,
    /// Memory reservation in bytes.
    pub memory_reservation: Option<u64>,
    /// Kernel memory limit in bytes.
    pub kernel_memory: Option<u64>,
}

impl Default for ResourceConstraints {
    fn default() -> Self {
        Self {
            cpu_shares: None,
            cpu_quota: None,
            cpu_period: None,
            cpus: None,
            cpuset_cpus: None,
            memory: None,
            memory_swap: None,
            memory_reservation: None,
            kernel_memory: None,
        }
    }
}

/// Port mapping specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port (0 for random).
    pub host_port: u16,
    /// Container port.
    pub container_port: u16,
    /// Protocol (tcp/udp).
    pub protocol: String,
    /// Host IP to bind to.
    pub host_ip: Option<String>,
}

/// Volume mount specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Host path or volume name.
    pub source: String,
    /// Container path.
    pub destination: String,
    /// Whether the mount is read-only.
    pub read_only: bool,
    /// Volume options.
    pub options: HashMap<String, String>,
}

/// Network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network mode (bridge, host, none, container:<name>).
    pub network_mode: String,
    /// Network aliases.
    pub aliases: Vec<String>,
    /// Port mappings.
    pub port_mappings: Vec<PortMapping>,
    /// Extra hosts.
    pub extra_hosts: Vec<String>,
    /// DNS servers.
    pub dns: Vec<String>,
    /// DNS search domains.
    pub dns_search: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            network_mode: "bridge".to_string(),
            aliases: Vec::new(),
            port_mappings: Vec::new(),
            extra_hosts: Vec::new(),
            dns: Vec::new(),
            dns_search: Vec::new(),
        }
    }
}

/// Container status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainerStatus {
    /// Container is created but not started.
    Created,
    /// Container is running.
    Running,
    /// Container is paused.
    Paused,
    /// Container is restarting.
    Restarting,
    /// Container is stopped.
    Stopped,
    /// Container is dead.
    Dead,
    /// Container status is unknown.
    Unknown,
}

/// Container information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    /// Container ID.
    pub id: String,
    /// Container name.
    pub name: String,
    /// Container image.
    pub image: String,
    /// Container status.
    pub status: ContainerStatus,
    /// When the container was created.
    pub created: chrono::DateTime<chrono::Utc>,
    /// When the container was started.
    pub started: Option<chrono::DateTime<chrono::Utc>>,
    /// When the container finished.
    pub finished: Option<chrono::DateTime<chrono::Utc>>,
    /// Exit code (if finished).
    pub exit_code: Option<i64>,
    /// Container labels.
    pub labels: HashMap<String, String>,
    /// Container annotations.
    pub annotations: HashMap<String, String>,
}

/// Image information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Image ID.
    pub id: String,
    /// Image repository.
    pub repository: String,
    /// Image tag.
    pub tag: String,
    /// Image digest.
    pub digest: Option<String>,
    /// When the image was created.
    pub created: chrono::DateTime<chrono::Utc>,
    /// Image size in bytes.
    pub size: u64,
    /// Image labels.
    pub labels: HashMap<String, String>,
}

/// Docker configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Docker host (e.g., "unix:///var/run/docker.sock").
    pub host: String,
    /// Docker API version.
    pub version: Option<String>,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to use TLS.
    pub tls: bool,
    /// TLS certificate path.
    pub cert_path: Option<PathBuf>,
    /// TLS key path.
    pub key_path: Option<PathBuf>,
    /// CA certificate path.
    pub ca_path: Option<PathBuf>,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            host: if cfg!(windows) {
                "npipe:////./pipe/docker_engine".to_string()
            } else {
                "unix:///var/run/docker.sock".to_string()
            },
            version: None,
            timeout_secs: 120,
            tls: false,
            cert_path: None,
            key_path: None,
            ca_path: None,
        }
    }
}

/// Containerd configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerdConfig {
    /// Containerd socket path.
    pub socket_path: PathBuf,
    /// Namespace.
    pub namespace: String,
    /// Timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for ContainerdConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/run/containerd/containerd.sock"),
            namespace: "default".to_string(),
            timeout_secs: 120,
        }
    }
}

/// Container runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime type.
    pub runtime_type: RuntimeType,
    /// Docker configuration (if using Docker).
    pub docker_config: Option<DockerConfig>,
    /// Containerd configuration (if using containerd).
    pub containerd_config: Option<ContainerdConfig>,
    /// Default resource constraints.
    pub default_resources: ResourceConstraints,
    /// Default network configuration.
    pub default_network: NetworkConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_type: RuntimeType::Docker,
            docker_config: Some(DockerConfig::default()),
            containerd_config: None,
            default_resources: ResourceConstraints::default(),
            default_network: NetworkConfig::default(),
        }
    }
}

/// Container health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Command to run for health check.
    pub test: Vec<String>,
    /// Time to wait between checks in seconds.
    pub interval_secs: u64,
    /// Timeout for health check in seconds.
    pub timeout_secs: u64,
    /// Number of retries before marking as unhealthy.
    pub retries: u32,
    /// Time to wait before starting health checks in seconds.
    pub start_period_secs: u64,
}

/// Container log configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log driver (json-file, syslog, journald, etc.).
    pub driver: String,
    /// Log driver options.
    pub options: HashMap<String, String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            driver: "json-file".to_string(),
            options: HashMap::new(),
        }
    }
}

/// Container restart policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    /// Restart policy name (no, on-failure, always, unless-stopped).
    pub name: String,
    /// Maximum retry count (for on-failure).
    pub maximum_retry_count: Option<u32>,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            name: "unless-stopped".to_string(),
            maximum_retry_count: None,
        }
    }
}

/// Container statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    /// CPU usage in nanoseconds.
    pub cpu_usage: u64,
    /// Memory usage in bytes.
    pub memory_usage: u64,
    /// Memory limit in bytes.
    pub memory_limit: u64,
    /// Network RX bytes.
    pub network_rx: u64,
    /// Network TX bytes.
    pub network_tx: u64,
    /// Block I/O read bytes.
    pub block_read: u64,
    /// Block I/O write bytes.
    pub block_write: u64,
    /// Number of processes.
    pub pids: u64,
    /// When the stats were collected.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}