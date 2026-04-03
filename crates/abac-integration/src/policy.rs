//! Policy engine for ABAC.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;

use crate::error::{AbacError, Result};
use crate::model::{Policy, PolicyRule, Subject, Resource, Environment};
use crate::evaluator::PolicyEvaluator;

/// Policy engine that stores and evaluates policies.
pub struct PolicyEngine {
    policies: Arc<DashMap<uuid::Uuid, Policy>>,
    rule_index: Arc<RwLock<HashMap<String, Vec<uuid::Uuid>>>>,
}

impl PolicyEngine {
    /// Create a new policy engine.
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            rule_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a policy.
    pub async fn add_policy(&self, policy: Policy) -> Result<()> {
        self.policies.insert(policy.id, policy.clone());
        // Index by name
        let mut index = self.rule_index.write().await;
        index.entry(policy.name.clone()).or_insert_with(Vec::new).push(policy.id);
        Ok(())
    }

    /// Remove a policy.
    pub async fn remove_policy(&self, policy_id: uuid::Uuid) -> Result<()> {
        if let Some((_, policy)) = self.policies.remove(&policy_id) {
            let mut index = self.rule_index.write().await;
            if let Some(ids) = index.get_mut(&policy.name) {
                ids.retain(|id| *id != policy_id);
                if ids.is_empty() {
                    index.remove(&policy.name);
                }
            }
        }
        Ok(())
    }

    /// Get a policy by ID.
    pub fn get_policy(&self, policy_id: uuid::Uuid) -> Option<Policy> {
        self.policies.get(&policy_id).map(|entry| entry.clone())
    }

    /// Find policies by name.
    pub async fn find_policies_by_name(&self, name: &str) -> Vec<Policy> {
        let index = self.rule_index.read().await;
        let ids = index.get(name).cloned().unwrap_or_default();
        ids.into_iter()
            .filter_map(|id| self.policies.get(&id).map(|entry| entry.clone()))
            .collect()
    }

    /// Evaluate a request against all policies.
    pub async fn evaluate(
        &self,
        subject: &Subject,
        resource: &Resource,
        action: &str,
        environment: &Environment,
    ) -> Result<bool> {
        let mut allowed = false;
        for entry in self.policies.iter() {
            let policy = entry.value();
            if self.evaluate_policy(policy, subject, resource, action, environment).await? {
                // If any policy allows, we allow (unless a deny rule overrides).
                // Simple implementation: first matching rule decides.
                allowed = true;
            }
        }
        Ok(allowed)
    }

    /// Evaluate a single policy.
    async fn evaluate_policy(
        &self,
        policy: &Policy,
        subject: &Subject,
        resource: &Resource,
        action: &str,
        environment: &Environment,
    ) -> Result<bool> {
        for rule in &policy.rules {
            if self.evaluate_rule(rule, subject, resource, action, environment).await? {
                return Ok(rule.effect == "allow");
            }
        }
        Ok(false)
    }

    /// Evaluate a single rule.
    async fn evaluate_rule(
        &self,
        rule: &PolicyRule,
        subject: &Subject,
        resource: &Resource,
        action: &str,
        environment: &Environment,
    ) -> Result<bool> {
        // Use the PolicyEvaluator for actual condition evaluation
        let evaluator = PolicyEvaluator;
        evaluator.evaluate_rule(rule, subject, resource, action, environment)
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_policy_engine_add() {
        let engine = PolicyEngine::new();
        let policy = Policy::new("test", "test policy");
        engine.add_policy(policy).await.unwrap();
        assert_eq!(engine.policies.len(), 1);
    }
}