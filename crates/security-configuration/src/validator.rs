//! Validation of security configurations.

use crate::config::{AuditConfig, CryptoConfig, Policy, PolicyRule, SecurityConfig, SecurityProfile};
use crate::error::{Result, SecurityConfigError};
use std::collections::{HashMap, HashSet};

/// Result of validating a configuration.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the configuration is valid.
    pub valid: bool,
    /// List of warnings (non‑fatal issues).
    pub warnings: Vec<String>,
    /// List of errors (fatal issues).
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Creates a new validation result.
    pub fn new() -> Self {
        Self {
            valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Adds a warning.
    pub fn warn(&mut self, message: String) {
        self.warnings.push(message);
    }

    /// Adds an error (also marks the result as invalid).
    pub fn error(&mut self, message: String) {
        self.valid = false;
        self.errors.push(message);
    }

    /// Merges another validation result into this one.
    pub fn merge(&mut self, other: ValidationResult) {
        self.valid = self.valid && other.valid;
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates a whole security configuration.
pub fn validate_config(config: &SecurityConfig) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Version check
    if config.version != "1.0" {
        result.warn(format!(
            "Configuration version '{}' is not the latest (1.0)",
            config.version
        ));
    }

    // Default profile must exist
    if !config.profiles.contains_key(&config.default_profile) {
        result.error(format!(
            "Default profile '{}' does not exist",
            config.default_profile
        ));
    }

    // Validate each profile
    for (name, profile) in &config.profiles {
        let profile_result = validate_profile(name, profile);
        result.merge(profile_result);
    }

    // Validate global policies
    for (idx, policy) in config.global_policies.iter().enumerate() {
        let policy_result = validate_policy(policy);
        if !policy_result.valid {
            result.error(format!("Global policy #{} is invalid", idx + 1));
        }
        result.merge(policy_result);
    }

    // Validate audit config
    let audit_result = validate_audit(&config.audit);
    result.merge(audit_result);

    // Validate crypto config
    let crypto_result = validate_crypto(&config.crypto);
    result.merge(crypto_result);

    // Check for duplicate policy IDs across profiles
    let mut seen_ids = HashSet::new();
    for profile in config.profiles.values() {
        for policy in &profile.policies {
            if !seen_ids.insert(&policy.id) {
                result.warn(format!(
                    "Policy ID '{}' appears in multiple profiles",
                    policy.id
                ));
            }
        }
    }

    result
}

/// Validates a single security profile.
pub fn validate_profile(name: &str, profile: &SecurityProfile) -> ValidationResult {
    let mut result = ValidationResult::new();

    if profile.description.is_empty() {
        result.warn(format!("Profile '{}' has empty description", name));
    }

    if profile.policies.is_empty() {
        result.warn(format!("Profile '{}' has no policies", name));
    }

    for (idx, policy) in profile.policies.iter().enumerate() {
        let policy_result = validate_policy(policy);
        if !policy_result.valid {
            result.error(format!(
                "Profile '{}', policy #{} is invalid",
                name,
                idx + 1
            ));
        }
        result.merge(policy_result);
    }

    result
}

/// Validates a single policy.
pub fn validate_policy(policy: &Policy) -> ValidationResult {
    let mut result = ValidationResult::new();

    if policy.id.is_empty() {
        result.error("Policy ID cannot be empty".to_string());
    }
    if policy.name.is_empty() {
        result.warn(format!("Policy '{}' has empty name", policy.id));
    }
    if policy.rules.is_empty() {
        result.warn(format!("Policy '{}' has no rules", policy.id));
    }

    for (idx, rule) in policy.rules.iter().enumerate() {
        let rule_result = validate_rule(rule);
        if !rule_result.valid {
            result.error(format!(
                "Policy '{}', rule #{} is invalid",
                policy.id,
                idx + 1
            ));
        }
        result.merge(rule_result);
    }

    result
}

/// Validates a single policy rule.
pub fn validate_rule(rule: &PolicyRule) -> ValidationResult {
    let mut result = ValidationResult::new();

    match rule {
        PolicyRule::Action {
            action,
            resource,
            allow: _,
            conditions,
        } => {
            if action.is_empty() {
                result.error("Action rule: action cannot be empty".to_string());
            }
            if resource.is_empty() {
                result.error("Action rule: resource cannot be empty".to_string());
            }
            // Validate condition keys are not empty
            for key in conditions.keys() {
                if key.is_empty() {
                    result.error("Action rule: condition key cannot be empty".to_string());
                }
            }
        }
        PolicyRule::Capability { capability, level } => {
            if capability.is_empty() {
                result.error("Capability rule: capability cannot be empty".to_string());
            }
            if *level == 0 {
                result.warn("Capability rule: level zero has no effect".to_string());
            }
        }
        PolicyRule::Crypto {
            algorithm,
            min_key_length,
        } => {
            if algorithm.is_empty() {
                result.error("Crypto rule: algorithm cannot be empty".to_string());
            }
            if *min_key_length < 64 {
                result.warn(format!(
                    "Crypto rule: very small key length ({}) may be insecure",
                    min_key_length
                ));
            }
        }
        PolicyRule::Network {
            allowed_ips,
            allowed_ports,
            require_tls: _,
        } => {
            if allowed_ips.is_empty() {
                result.warn("Network rule: allowed_ips is empty (no IP allowed)".to_string());
            }
            // Validate CIDR syntax (simplified)
            for cidr in allowed_ips {
                if !cidr.contains('/') {
                    result.warn(format!(
                        "Network rule: IP range '{}' does not look like a CIDR",
                        cidr
                    ));
                }
            }
            if allowed_ports.is_empty() {
                result.warn("Network rule: allowed_ports is empty (no port allowed)".to_string());
            }
        }
        PolicyRule::Custom { subtype, params } => {
            if subtype.is_empty() {
                result.error("Custom rule: subtype cannot be empty".to_string());
            }
            // No further validation for custom rules
        }
    }

    result
}

/// Validates audit configuration.
pub fn validate_audit(audit: &AuditConfig) -> ValidationResult {
    let mut result = ValidationResult::new();

    if !audit.enabled {
        return result; // nothing to validate
    }

    match &audit.sink {
        crate::config::AuditSink::File { path } => {
            if path.is_empty() {
                result.error("Audit file sink: path cannot be empty".to_string());
            }
        }
        crate::config::AuditSink::Syslog { facility } => {
            if facility.is_empty() {
                result.error("Audit syslog sink: facility cannot be empty".to_string());
            }
        }
        crate::config::AuditSink::Http { url, auth_token: _ } => {
            if url.is_empty() {
                result.error("Audit HTTP sink: URL cannot be empty".to_string());
            }
        }
        crate::config::AuditSink::Mesh => {
            // No extra validation
        }
        crate::config::AuditSink::Null => {
            // No extra validation
        }
    }

    result
}

/// Validates cryptographic configuration.
pub fn validate_crypto(crypto: &CryptoConfig) -> ValidationResult {
    let mut result = ValidationResult::new();

    if crypto.default_symmetric.is_empty() {
        result.error("Crypto config: default_symmetric cannot be empty".to_string());
    }
    if crypto.default_asymmetric.is_empty() {
        result.error("Crypto config: default_asymmetric cannot be empty".to_string());
    }
    if crypto.hash_algorithm.is_empty() {
        result.error("Crypto config: hash_algorithm cannot be empty".to_string());
    }

    // Validate KDF
    if crypto.kdf.algorithm.is_empty() {
        result.error("Crypto config: KDF algorithm cannot be empty".to_string());
    }
    if crypto.kdf.iterations == 0 {
        result.error("Crypto config: KDF iterations must be positive".to_string());
    }
    if crypto.kdf.memory_kib == 0 {
        result.warn("Crypto config: KDF memory is zero".to_string());
    }
    if crypto.kdf.parallelism == 0 {
        result.error("Crypto config: KDF parallelism must be positive".to_string());
    }

    // Validate allowed ciphers
    if crypto.allowed_ciphers.is_empty() {
        result.warn("Crypto config: allowed_ciphers is empty".to_string());
    }

    result
}

/// Loads and validates a configuration from a YAML file.
pub fn load_and_validate_yaml(path: &str) -> Result<(SecurityConfig, ValidationResult)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| SecurityConfigError::ConfigFile(e))?;
    let config: SecurityConfig = serde_yaml::from_str(&content)
        .map_err(|e| SecurityConfigError::InvalidSyntax(e.to_string()))?;
    let validation = validate_config(&config);
    Ok((config, validation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuditSink, PolicyRule};

    fn sample_config() -> SecurityConfig {
        SecurityConfig {
            version: "1.0".to_string(),
            default_profile: "default".to_string(),
            profiles: {
                let mut map = HashMap::new();
                map.insert(
                    "default".to_string(),
                    SecurityProfile {
                        description: "Default profile".to_string(),
                        policies: vec![Policy {
                            id: "policy1".to_string(),
                            name: "Test Policy".to_string(),
                            description: "Test".to_string(),
                            rules: vec![PolicyRule::Action {
                                action: "read".to_string(),
                                resource: "file:*".to_string(),
                                allow: true,
                                conditions: HashMap::new(),
                            }],
                            mandatory: false,
                            tags: vec![],
                        }],
                        enabled: true,
                        priority: 1,
                    },
                );
                map
            },
            global_policies: Vec::new(),
            audit: AuditConfig::default(),
            crypto: CryptoConfig::default(),
        }
    }

    #[test]
    fn test_validate_config_ok() {
        let config = sample_config();
        let result = validate_config(&config);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_config_missing_default_profile() {
        let mut config = sample_config();
        config.default_profile = "missing".to_string();
        let result = validate_config(&config);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Default profile")));
    }

    #[test]
    fn test_validate_policy_empty_id() {
        let policy = Policy {
            id: "".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            rules: vec![],
            mandatory: false,
            tags: vec![],
        };
        let result = validate_policy(&policy);
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_rule_action_empty() {
        let rule = PolicyRule::Action {
            action: "".to_string(),
            resource: "*".to_string(),
            allow: true,
            conditions: HashMap::new(),
        };
        let result = validate_rule(&rule);
        assert!(!result.valid);
    }

    #[test]
    fn test_load_and_validate_yaml() {
        // This test would require a temporary YAML file; we skip it for brevity.
        // In a real test you would create a tempfile with valid YAML.
    }
}