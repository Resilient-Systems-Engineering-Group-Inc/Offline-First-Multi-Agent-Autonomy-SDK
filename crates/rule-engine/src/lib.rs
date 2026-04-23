//! Business rule engine with DSL support.
//!
//! Provides:
//! - Rule definition and execution
//! - DSL for rule expressions
//! - Rule chaining and priority
//! - Conflict resolution
//! - Audit logging

pub mod rule;
pub mod engine;
pub mod dsl;
pub mod actions;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use rule::*;
pub use engine::*;
pub use dsl::*;
pub use actions::*;

/// Rule engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngineConfig {
    pub max_rules: usize,
    pub max_chain_depth: usize,
    pub enable_audit: bool,
    pub conflict_resolution: ConflictResolution,
    pub default_priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    Priority,
    FirstMatch,
    AllMatch,
}

impl Default for RuleEngineConfig {
    fn default() -> Self {
        Self {
            max_rules: 1000,
            max_chain_depth: 10,
            enable_audit: true,
            conflict_resolution: ConflictResolution::Priority,
            default_priority: 50,
        }
    }
}

/// Rule engine manager.
pub struct RuleEngineManager {
    config: RuleEngineConfig,
    engine: RuleEngine,
    rules: RwLock<HashMap<String, Rule>>,
    rule_sets: RwLock<HashMap<String, RuleSet>>,
    audit_log: RwLock<Vec<AuditEntry>>,
}

impl RuleEngineManager {
    /// Create new rule engine manager.
    pub fn new(config: RuleEngineConfig) -> Self {
        Self {
            config,
            engine: RuleEngine::new(&config),
            rules: RwLock::new(HashMap::new()),
            rule_sets: RwLock::new(HashMap::new()),
            audit_log: RwLock::new(Vec::new()),
        }
    }

    /// Initialize rule engine.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing rule engine with max rules: {}", self.config.max_rules);
        Ok(())
    }

    /// Register rule.
    pub async fn register_rule(&self, rule: Rule) -> Result<()> {
        let mut rules = self.rules.write().await;
        
        if rules.len() >= self.config.max_rules {
            return Err(anyhow::anyhow!("Maximum rule limit reached"));
        }

        let rule_id = rule.id.clone();
        rules.insert(rule_id.clone(), rule);

        info!("Rule registered: {}", rule_id);
        Ok(())
    }

    /// Register rule from DSL.
    pub async fn register_rule_dsl(&self, dsl: &str) -> Result<String> {
        let rule = RuleParser::parse_rule(dsl)?;
        let rule_id = rule.id.clone();
        self.register_rule(rule).await?;
        Ok(rule_id)
    }

    /// Create rule set.
    pub async fn create_rule_set(&self, name: &str, rule_ids: Vec<String>) -> Result<()> {
        let mut rule_sets = self.rule_sets.write().await;
        
        let rule_set = RuleSet {
            name: name.to_string(),
            rule_ids,
            enabled: true,
        };

        rule_sets.insert(name.to_string(), rule_set);
        info!("Rule set created: {}", name);
        Ok(())
    }

    /// Execute rules.
    pub async fn execute(&self, context: &RuleContext) -> Result<RuleResult> {
        let rules = self.rules.read().await;
        let rule_sets = self.rule_sets.read().await;

        let result = self.engine.execute(&rules, &rule_sets, context).await?;

        // Audit logging
        if self.config.enable_audit {
            self.log_audit(context, &result).await;
        }

        Ok(result)
    }

    /// Execute specific rule set.
    pub async fn execute_rule_set(&self, set_name: &str, context: &RuleContext) -> Result<RuleResult> {
        let rules = self.rules.read().await;
        let rule_sets = self.rule_sets.read().await;

        let rule_set = rule_sets.get(set_name)
            .ok_or_else(|| anyhow::anyhow!("Rule set not found: {}", set_name))?;

        if !rule_set.enabled {
            return Err(anyhow::anyhow!("Rule set is disabled: {}", set_name));
        }

        let result = self.engine.execute_set(&rules, rule_set, context).await?;

        if self.config.enable_audit {
            self.log_audit(context, &result).await;
        }

        Ok(result)
    }

    /// Enable/disable rule.
    pub async fn toggle_rule(&self, rule_id: &str, enabled: bool) -> Result<()> {
        let mut rules = self.rules.write().await;
        
        let rule = rules.get_mut(rule_id)
            .ok_or_else(|| anyhow::anyhow!("Rule not found: {}", rule_id))?;

        rule.enabled = enabled;
        info!("Rule {} {}", rule_id, if enabled { "enabled" } else { "disabled" });
        Ok(())
    }

    /// Delete rule.
    pub async fn delete_rule(&self, rule_id: &str) -> Result<()> {
        self.rules.write().await.remove(rule_id);
        info!("Rule deleted: {}", rule_id);
        Ok(())
    }

    /// Get rule by ID.
    pub async fn get_rule(&self, rule_id: &str) -> Result<Rule> {
        let rules = self.rules.read().await;
        
        rules.get(rule_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Rule not found: {}", rule_id))
    }

    /// List all rules.
    pub async fn list_rules(&self) -> Vec<RuleSummary> {
        let rules = self.rules.read().await;
        rules.values().map(|r| r.summary()).collect()
    }

    /// Get audit log.
    pub async fn get_audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        let audit_log = self.audit_log.read().await;
        audit_log.iter().rev().take(limit).cloned().collect()
    }

    /// Clear audit log.
    pub async fn clear_audit_log(&self) {
        self.audit_log.write().await.clear();
    }

    /// Get engine statistics.
    pub async fn get_stats(&self) -> RuleEngineStats {
        let rules = self.rules.read().await;
        let rule_sets = self.rule_sets.read().await;
        let audit_log = self.audit_log.read().await;

        RuleEngineStats {
            total_rules: rules.len() as i32,
            enabled_rules: rules.values().filter(|r| r.enabled).count() as i32,
            total_rule_sets: rule_sets.len() as i32,
            audit_entries: audit_log.len() as i32,
        }
    }

    async fn log_audit(&self, context: &RuleContext, result: &RuleResult) {
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            context: context.clone(),
            rules_fired: result.fired_rules.clone(),
            actions_executed: result.actions.len() as i32,
            result: result.clone(),
        };

        let mut audit_log = self.audit_log.write().await;
        audit_log.push(entry);

        // Limit audit log size
        if audit_log.len() > 10000 {
            audit_log.drain(..5000);
        }
    }
}

/// Rule set - group of rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub name: String,
    pub rule_ids: Vec<String>,
    pub enabled: bool,
}

/// Rule execution context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    pub data: serde_json::Value,
    pub metadata: RuleMetadata,
}

impl RuleContext {
    pub fn new(data: serde_json::Value) -> Self {
        Self {
            data,
            metadata: RuleMetadata::default(),
        }
    }

    pub fn with_metadata(mut self, metadata: RuleMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn get(&self, path: &str) -> Option<&serde_json::Value> {
        self.data.pointer(path.trim_start_matches('/'))
    }
}

/// Rule metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleMetadata {
    pub source: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Rule execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub matched: bool,
    pub fired_rules: Vec<String>,
    pub actions: Vec<Action>,
    pub output: serde_json::Value,
    pub execution_time_ms: f64,
}

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: RuleContext,
    pub rules_fired: Vec<String>,
    pub actions_executed: i32,
    pub result: RuleResult,
}

/// Rule engine statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngineStats {
    pub total_rules: i32,
    pub enabled_rules: i32,
    pub total_rule_sets: i32,
    pub audit_entries: i32,
}

/// Rule summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummary {
    pub id: String,
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
    pub condition_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_engine() {
        let config = RuleEngineConfig::default();
        let manager = RuleEngineManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Create rule
        let rule = Rule::new("test-rule", "Test Rule")
            .with_condition(RuleCondition::Equals {
                field: "priority".to_string(),
                value: serde_json::json!("high"),
            })
            .with_action(Action::Set {
                field: "status".to_string(),
                value: serde_json::json!("approved"),
            })
            .with_priority(100);

        manager.register_rule(rule).await.unwrap();

        // Execute
        let context = RuleContext::new(serde_json::json!({
            "priority": "high",
            "amount": 1000
        }));

        let result = manager.execute(&context).await.unwrap();
        assert!(result.matched);
        assert_eq!(result.fired_rules.len(), 1);

        // Get stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_rules, 1);
        assert_eq!(stats.enabled_rules, 1);
    }
}
