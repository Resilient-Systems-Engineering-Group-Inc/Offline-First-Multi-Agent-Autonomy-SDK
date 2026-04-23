//! Rule engine execution.

use crate::{Rule, RuleCondition, RuleContext, RuleResult, RuleSet, RuleEngineConfig, Action};
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Rule engine.
pub struct RuleEngine {
    config: RuleEngineConfig,
    execution_count: RwLock<i64>,
}

impl RuleEngine {
    pub fn new(config: &RuleEngineConfig) -> Self {
        Self {
            config: config.clone(),
            execution_count: RwLock::new(0),
        }
    }

    /// Execute all matching rules.
    pub async fn execute(
        &self,
        rules: &HashMap<String, Rule>,
        _rule_sets: &HashMap<String, RuleSet>,
        context: &RuleContext,
    ) -> Result<RuleResult> {
        let start = std::time::Instant::now();
        
        let mut fired_rules = Vec::new();
        let mut actions = Vec::new();
        let mut output = context.data.clone();

        // Get enabled rules sorted by priority
        let mut enabled_rules: Vec<_> = rules.values()
            .filter(|r| r.enabled)
            .collect();
        
        enabled_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Execute rules
        for rule in enabled_rules {
            if rule.condition.evaluate(&context.data) {
                fired_rules.push(rule.id.clone());
                
                // Execute actions
                for action in &rule.actions {
                    let action_result = action.execute(&mut output).await?;
                    if action_result {
                        actions.push(action.clone());
                    }
                }

                // Check chain depth
                if fired_rules.len() >= self.config.max_chain_depth {
                    tracing::warn!("Max chain depth reached");
                    break;
                }

                // Conflict resolution
                match self.config.conflict_resolution {
                    crate::ConflictResolution::FirstMatch => break,
                    crate::ConflictResolution::Priority => continue,
                    crate::ConflictResolution::AllMatch => continue,
                }
            }
        }

        // Update execution count
        *self.execution_count.write().await += 1;

        let execution_time = start.elapsed().as_secs_f64() * 1000.0;

        Ok(RuleResult {
            matched: !fired_rules.is_empty(),
            fired_rules,
            actions,
            output,
            execution_time_ms: execution_time,
        })
    }

    /// Execute specific rule set.
    pub async fn execute_set(
        &self,
        rules: &HashMap<String, Rule>,
        rule_set: &RuleSet,
        context: &RuleContext,
    ) -> Result<RuleResult> {
        let start = std::time::Instant::now();
        
        let mut fired_rules = Vec::new();
        let mut actions = Vec::new();
        let mut output = context.data.clone();

        // Get rules from set
        for rule_id in &rule_set.rule_ids {
            if let Some(rule) = rules.get(rule_id) {
                if rule.enabled && rule.condition.evaluate(&context.data) {
                    fired_rules.push(rule.id.clone());
                    
                    for action in &rule.actions {
                        let action_result = action.execute(&mut output).await?;
                        if action_result {
                            actions.push(action.clone());
                        }
                    }

                    if fired_rules.len() >= self.config.max_chain_depth {
                        break;
                    }
                }
            }
        }

        *self.execution_count.write().await += 1;

        let execution_time = start.elapsed().as_secs_f64() * 1000.0;

        Ok(RuleResult {
            matched: !fired_rules.is_empty(),
            fired_rules,
            actions,
            output,
            execution_time_ms: execution_time,
        })
    }

    /// Get execution count.
    pub async fn get_execution_count(&self) -> i64 {
        *self.execution_count.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rule, RuleCondition, Action};

    #[tokio::test]
    async fn test_rule_engine_execute() {
        let config = RuleEngineConfig::default();
        let engine = RuleEngine::new(&config);

        let mut rules = HashMap::new();
        
        // Add test rule
        let rule = Rule::new("rule1", "Test Rule 1")
            .with_condition(RuleCondition::Equals {
                field: "status".to_string(),
                value: serde_json::json!("active"),
            })
            .with_action(Action::Set {
                field: "processed".to_string(),
                value: serde_json::json!(true),
            })
            .with_priority(100);

        rules.insert(rule.id.clone(), rule);

        let rule_sets = HashMap::new();
        let context = RuleContext::new(serde_json::json!({
            "status": "active",
            "count": 10
        }));

        let result = engine.execute(&rules, &rule_sets, &context).await.unwrap();
        
        assert!(result.matched);
        assert_eq!(result.fired_rules.len(), 1);
        assert_eq!(result.actions.len(), 1);
        assert!(result.output.get("processed").and_then(|v| v.as_bool()).unwrap());
    }
}
