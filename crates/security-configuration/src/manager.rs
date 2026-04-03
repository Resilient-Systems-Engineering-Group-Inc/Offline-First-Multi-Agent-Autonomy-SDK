//! Central manager for security configurations.

use crate::audit::{AuditEvent, AuditLogger, GlobalAuditLogger, Outcome};
use crate::config::{AuditConfig, SecurityConfig, SecurityProfile};
use crate::error::{Result, SecurityConfigError};
use crate::policy::{EvaluationContext, RuleDecision};
use crate::profile::ProfileManager;
use crate::validator::{validate_config, ValidationResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main security configuration manager.
///
/// This struct provides:
/// - Loading and validation of security configurations
/// - Hot‑reload of configuration files
/// - Policy evaluation
/// - Integration with audit logging
/// - Profile management
pub struct SecurityConfigManager {
    config: Arc<RwLock<SecurityConfig>>,
    profile_manager: Arc<RwLock<ProfileManager>>,
    audit_logger: Option<AuditLogger>,
    global_audit: GlobalAuditLogger,
    config_path: Option<String>,
}

impl SecurityConfigManager {
    /// Creates a new manager by loading configuration from a YAML file.
    pub async fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let (config, validation) =
            crate::validator::load_and_validate_yaml(&path_str)?;

        if !validation.valid {
            return Err(SecurityConfigError::ValidationFailed(format!(
                "Configuration validation failed: {:?}",
                validation.errors
            )));
        }

        let profile_manager = ProfileManager::from(config.profiles.clone());
        let audit_logger = if config.audit.enabled {
            Some(AuditLogger::new(config.audit.clone())?)
        } else {
            None
        };

        let global_audit = GlobalAuditLogger::new();
        if config.audit.enabled {
            global_audit.init(config.audit.clone()).await?;
        }

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            profile_manager: Arc::new(RwLock::new(profile_manager)),
            audit_logger,
            global_audit,
            config_path: Some(path_str),
        })
    }

    /// Creates a new manager from an already‑parsed configuration.
    pub async fn from_config(config: SecurityConfig) -> Result<Self> {
        let validation = validate_config(&config);
        if !validation.valid {
            return Err(SecurityConfigError::ValidationFailed(format!(
                "Configuration validation failed: {:?}",
                validation.errors
            )));
        }

        let profile_manager = ProfileManager::from(config.profiles.clone());
        let audit_logger = if config.audit.enabled {
            Some(AuditLogger::new(config.audit.clone())?)
        } else {
            None
        };

        let global_audit = GlobalAuditLogger::new();
        if config.audit.enabled {
            global_audit.init(config.audit.clone()).await?;
        }

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            profile_manager: Arc::new(RwLock::new(profile_manager)),
            audit_logger,
            global_audit,
            config_path: None,
        })
    }

    /// Returns the current configuration (read‑only).
    pub async fn get_config(&self) -> SecurityConfig {
        self.config.read().await.clone()
    }

    /// Returns the profile manager.
    pub async fn get_profile_manager(&self) -> ProfileManager {
        self.profile_manager.read().await.clone()
    }

    /// Returns a specific profile by name.
    pub async fn get_profile(&self, name: Option<&str>) -> Result<SecurityProfile> {
        let manager = self.profile_manager.read().await;
        let profile = manager.get_profile(name)?;
        Ok(profile.clone())
    }

    /// Evaluates an action against the current security configuration.
    ///
    /// This method evaluates the action against all policies in the specified profile
    /// (or the default profile) and returns the final decision.
    pub async fn evaluate(
        &self,
        ctx: EvaluationContext,
        profile_name: Option<&str>,
    ) -> Result<(RuleDecision, Vec<String>)> {
        let profile = self.get_profile(profile_name).await?;
        let policies: Vec<_> = profile.policies.iter().collect();
        let (decision, individual) = crate::policy::evaluate_policies(&policies, &ctx);

        // Log the evaluation as an audit event if audit is enabled.
        let outcome = match decision {
            RuleDecision::Allow => Outcome::Allowed,
            RuleDecision::Deny => Outcome::Denied,
            RuleDecision::NotApplicable => Outcome::Success,
        };
        let event = AuditEvent::new(
            crate::config::AuditSeverity::Info,
            "security_config",
            "policy_evaluation",
            ctx.resource.clone(),
            outcome,
        )
        .with_agent_id(ctx.agent_id.unwrap_or_default())
        .with_detail("action", serde_json::Value::String(ctx.action.clone()))
        .with_detail("profile", serde_json::Value::String(profile_name.unwrap_or("default").to_string()))
        .with_detail("decision", serde_json::Value::String(format!("{:?}", decision)));

        self.global_audit.log(event).await?;

        let explanations = individual
            .iter()
            .map(|d| d.explanation.clone())
            .collect();

        Ok((decision, explanations))
    }

    /// Reloads the configuration from the original file (if any).
    pub async fn reload(&mut self) -> Result<()> {
        let path = match &self.config_path {
            Some(p) => p,
            None => {
                return Err(SecurityConfigError::ConfigFile(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No configuration file path known",
                )));
            }
        };

        let (new_config, validation) = crate::validator::load_and_validate_yaml(path)?;
        if !validation.valid {
            return Err(SecurityConfigError::ValidationFailed(format!(
                "Reload validation failed: {:?}",
                validation.errors
            )));
        }

        // Update the configuration
        *self.config.write().await = new_config.clone();
        *self.profile_manager.write().await = ProfileManager::from(new_config.profiles.clone());

        // Update audit logger
        if new_config.audit.enabled {
            self.global_audit.init(new_config.audit.clone()).await?;
            self.audit_logger = Some(AuditLogger::new(new_config.audit.clone())?);
        } else {
            self.audit_logger = None;
        }

        // Log the reload event
        let event = AuditEvent::new(
            crate::config::AuditSeverity::Info,
            "security_config",
            "config_reloaded",
            path,
            Outcome::Success,
        );
        self.global_audit.log(event).await?;

        Ok(())
    }

    /// Updates the configuration in‑memory (does not persist to file).
    pub async fn update_config(&self, new_config: SecurityConfig) -> Result<()> {
        let validation = validate_config(&new_config);
        if !validation.valid {
            return Err(SecurityConfigError::ValidationFailed(format!(
                "Update validation failed: {:?}",
                validation.errors
            )));
        }

        *self.config.write().await = new_config.clone();
        *self.profile_manager.write().await = ProfileManager::from(new_config.profiles.clone());

        // Update audit logger
        if new_config.audit.enabled {
            self.global_audit.init(new_config.audit.clone()).await?;
        }

        Ok(())
    }

    /// Returns the audit logger (if enabled).
    pub fn audit_logger(&self) -> Option<&AuditLogger> {
        self.audit_logger.as_ref()
    }

    /// Returns the global audit logger.
    pub fn global_audit(&self) -> &GlobalAuditLogger {
        &self.global_audit
    }

    /// Returns the path of the configuration file, if any.
    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref()
    }
}

impl Default for SecurityConfigManager {
    fn default() -> Self {
        let config = SecurityConfig::default();
        let profile_manager = ProfileManager::from(config.profiles.clone());
        let global_audit = GlobalAuditLogger::new();
        Self {
            config: Arc::new(RwLock::new(config)),
            profile_manager: Arc::new(RwLock::new(profile_manager)),
            audit_logger: None,
            global_audit,
            config_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Policy, PolicyRule};
    use std::collections::HashMap;

    fn sample_config() -> SecurityConfig {
        SecurityConfig {
            version: "1.0".to_string(),
            default_profile: "default".to_string(),
            profiles: {
                let mut map = HashMap::new();
                map.insert(
                    "default".to_string(),
                    SecurityProfile {
                        description: "Default".to_string(),
                        policies: vec![Policy {
                            id: "p1".to_string(),
                            name: "Test".to_string(),
                            description: "".to_string(),
                            rules: vec![PolicyRule::Action {
                                action: "read".to_string(),
                                resource: "*".to_string(),
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
            crypto: crate::config::CryptoConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_manager_from_config() {
        let config = sample_config();
        let manager = SecurityConfigManager::from_config(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_get_profile() {
        let config = sample_config();
        let manager = SecurityConfigManager::from_config(config).await.unwrap();
        let profile = manager.get_profile(None).await;
        assert!(profile.is_ok());
        assert_eq!(profile.unwrap().description, "Default");
    }

    #[tokio::test]
    async fn test_evaluate() {
        let config = sample_config();
        let manager = SecurityConfigManager::from_config(config).await.unwrap();
        let ctx = EvaluationContext {
            action: "read".to_string(),
            resource: "file:test".to_string(),
            ..Default::default()
        };
        let (decision, _) = manager.evaluate(ctx, None).await.unwrap();
        assert_eq!(decision, RuleDecision::Allow);
    }

    #[tokio::test]
    async fn test_update_config() {
        let config = sample_config();
        let manager = SecurityConfigManager::from_config(config).await.unwrap();
        let mut new_config = sample_config();
        new_config.default_profile = "new".to_string();
        // This should fail because profile "new" doesn't exist
        let result = manager.update_config(new_config).await;
        assert!(result.is_err());
    }
}