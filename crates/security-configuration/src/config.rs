//! Core configuration structures for security management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top‑level security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Version of the configuration schema.
    pub version: String,
    /// Default profile to use when none is specified.
    pub default_profile: String,
    /// Map of profile name → security profile.
    pub profiles: HashMap<String, SecurityProfile>,
    /// Global policies that apply to all profiles.
    #[serde(default)]
    pub global_policies: Vec<Policy>,
    /// Settings for audit logging.
    #[serde(default)]
    pub audit: AuditConfig,
    /// Cryptographic settings (optional).
    #[serde(default)]
    pub crypto: CryptoConfig,
}

/// Security profile – a named set of policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProfile {
    /// Human‑readable description.
    pub description: String,
    /// List of policies active in this profile.
    pub policies: Vec<Policy>,
    /// Whether this profile is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Priority (higher = more restrictive).
    #[serde(default)]
    pub priority: u8,
}

/// A security policy with a set of rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique identifier of the policy.
    pub id: String,
    /// Human‑readable name.
    pub name: String,
    /// Description of what the policy enforces.
    pub description: String,
    /// The rules that define the policy.
    pub rules: Vec<PolicyRule>,
    /// Whether the policy is mandatory (cannot be overridden).
    #[serde(default)]
    pub mandatory: bool,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A single rule within a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PolicyRule {
    /// Allow or deny a specific action.
    Action {
        /// Action identifier (e.g., "send_message", "read_file").
        action: String,
        /// Resource pattern (e.g., "agent:*", "file:/etc/*").
        resource: String,
        /// Whether to allow (true) or deny (false).
        allow: bool,
        /// Conditions that must be satisfied.
        #[serde(default)]
        conditions: HashMap<String, String>,
    },
    /// Require a specific capability.
    Capability {
        /// Capability name.
        capability: String,
        /// Minimum level required.
        level: u8,
    },
    /// Enforce a cryptographic algorithm.
    Crypto {
        /// Algorithm name (e.g., "AES‑256‑GCM", "Ed25519").
        algorithm: String,
        /// Minimum key length in bits.
        min_key_length: u32,
    },
    /// Network‑related restriction.
    Network {
        /// Allowed IP ranges (CIDR).
        allowed_ips: Vec<String>,
        /// Allowed ports.
        allowed_ports: Vec<u16>,
        /// Whether to require TLS.
        require_tls: bool,
    },
    /// Custom rule with arbitrary key‑value parameters.
    Custom {
        /// Rule subtype identifier.
        subtype: String,
        /// Parameters.
        params: HashMap<String, serde_json::Value>,
    },
}

/// Configuration for audit logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled.
    pub enabled: bool,
    /// Where to send audit events (file, syslog, remote).
    pub sink: AuditSink,
    /// Minimum severity level to log.
    pub min_severity: AuditSeverity,
    /// Whether to log successful security decisions.
    #[serde(default = "default_true")]
    pub log_success: bool,
}

/// Destination for audit events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditSink {
    /// Write to a local file.
    File { path: String },
    /// Send to a syslog daemon.
    Syslog { facility: String },
    /// Send to a remote HTTP endpoint.
    Http { url: String, auth_token: Option<String> },
    /// Send via the mesh transport.
    Mesh,
    /// Discard events (no‑op).
    Null,
}

/// Severity levels for audit events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Debug information.
    Debug,
    /// Normal operational events.
    Info,
    /// Notable events that may require attention.
    Notice,
    /// Security‑relevant events.
    Warning,
    /// Security violations that were blocked.
    Error,
    /// Critical security incidents.
    Critical,
}

/// Cryptographic configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    /// Default symmetric encryption algorithm.
    pub default_symmetric: String,
    /// Default asymmetric signature algorithm.
    pub default_asymmetric: String,
    /// Hash algorithm for integrity checks.
    pub hash_algorithm: String,
    /// Key derivation function settings.
    #[serde(default)]
    pub kdf: KdfConfig,
    /// Whether to enforce forward secrecy.
    #[serde(default)]
    pub forward_secrecy: bool,
    /// Allowed cipher suites.
    #[serde(default)]
    pub allowed_ciphers: Vec<String>,
}

/// Key derivation function configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfConfig {
    /// Algorithm (e.g., "argon2id", "pbkdf2").
    pub algorithm: String,
    /// Iteration count.
    pub iterations: u32,
    /// Memory size in KiB.
    pub memory_kib: u32,
    /// Parallelism factor.
    pub parallelism: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            default_profile: "default".to_string(),
            profiles: HashMap::new(),
            global_policies: Vec::new(),
            audit: AuditConfig::default(),
            crypto: CryptoConfig::default(),
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sink: AuditSink::Null,
            min_severity: AuditSeverity::Info,
            log_success: true,
        }
    }
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            default_symmetric: "AES‑256‑GCM".to_string(),
            default_asymmetric: "Ed25519".to_string(),
            hash_algorithm: "SHA‑256".to_string(),
            kdf: KdfConfig::default(),
            forward_secrecy: true,
            allowed_ciphers: vec![
                "AES‑256‑GCM".to_string(),
                "ChaCha20‑Poly1305".to_string(),
            ],
        }
    }
}

impl Default for KdfConfig {
    fn default() -> Self {
        Self {
            algorithm: "argon2id".to_string(),
            iterations: 3,
            memory_kib: 4096,
            parallelism: 1,
        }
    }
}

/// Helper function for serde defaults.
fn default_true() -> bool {
    true
}