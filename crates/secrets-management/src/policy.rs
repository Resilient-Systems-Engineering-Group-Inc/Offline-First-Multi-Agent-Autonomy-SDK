//! Access policy engine for secrets.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::error::{SecretsError, Result};

/// Policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
    AllowWithAudit,
}

/// Policy rule condition.
#[derive(Debug, Clone)]
pub enum Condition {
    /// Agent ID matches.
    AgentId(u64),
    
    /// Agent has capability.
    HasCapability(String),
    
    /// Time is within window (start_hour, end_hour in UTC).
    TimeWindow(u8, u8),
    
    /// Access count less than limit.
    MaxAccesses(u32),
    
    /// Secret has tag.
    HasTag(String),
    
    /// Secret metadata matches.
    MetadataEquals(String, String),
    
    /// Combined conditions (AND).
    All(Vec<Condition>),
    
    /// Combined conditions (OR).
    Any(Vec<Condition>),
    
    /// Negated condition.
    Not(Box<Condition>),
}

/// Policy rule.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Rule ID.
    pub id: String,
    
    /// Description.
    pub description: String,
    
    /// Conditions that must be satisfied.
    pub conditions: Vec<Condition>,
    
    /// The decision if conditions are met.
    pub decision: PolicyDecision,
    
    /// Priority (higher = evaluated first).
    pub priority: i32,
}

impl PolicyRule {
    /// Create a new policy rule.
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            conditions: Vec::new(),
            decision: PolicyDecision::Deny,
            priority: 0,
        }
    }
    
    /// Add a condition.
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }
    
    /// Set decision.
    pub fn with_decision(mut self, decision: PolicyDecision) -> Self {
        self.decision = decision;
        self
    }
    
    /// Set priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// Evaluate the rule against a context.
    pub fn evaluate(&self, context: &PolicyContext) -> Option<PolicyDecision> {
        for condition in &self.conditions {
            if !condition.evaluate(context) {
                return None; // Condition not met
            }
        }
        
        Some(self.decision)
    }
}

impl Condition {
    /// Evaluate the condition against a context.
    pub fn evaluate(&self, context: &PolicyContext) -> bool {
        match self {
            Condition::AgentId(id) => context.agent_id == *id,
            Condition::HasCapability(cap) => context.capabilities.contains(cap),
            Condition::TimeWindow(start, end) => {
                use chrono::Timelike;
                let now = chrono::Utc::now();
                let hour = now.hour() as u8;
                hour >= *start && hour < *end
            }
            Condition::MaxAccesses(max) => context.access_count < *max,
            Condition::HasTag(tag) => context.secret_tags.contains(tag),
            Condition::MetadataEquals(key, value) => {
                context.secret_metadata.get(key).map(|v| v == value).unwrap_or(false)
            }
            Condition::All(conditions) => {
                conditions.iter().all(|c| c.evaluate(context))
            }
            Condition::Any(conditions) => {
                conditions.iter().any(|c| c.evaluate(context))
            }
            Condition::Not(condition) => !condition.evaluate(context),
        }
    }
}

/// Context for policy evaluation.
#[derive(Debug, Clone, Default)]
pub struct PolicyContext {
    /// Agent ID attempting access.
    pub agent_id: u64,
    
    /// Agent capabilities.
    pub capabilities: Vec<String>,
    
    /// Secret ID being accessed.
    pub secret_id: String,
    
    /// Secret tags.
    pub secret_tags: Vec<String>,
    
    /// Secret metadata.
    pub secret_metadata: HashMap<String, String>,
    
    /// Access count for this secret.
    pub access_count: u32,
    
    /// Operation type (read, write, delete, etc.)
    pub operation: String,
    
    /// Timestamp of access attempt.
    pub timestamp: u64,
}

impl PolicyContext {
    /// Create a new policy context.
    pub fn new(agent_id: u64, secret_id: impl Into<String>) -> Self {
        Self {
            agent_id,
            secret_id: secret_id.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ..Default::default()
        }
    }
    
    /// Add a capability.
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }
    
    /// Add a secret tag.
    pub fn with_secret_tag(mut self, tag: impl Into<String>) -> Self {
        self.secret_tags.push(tag.into());
        self
    }
    
    /// Add secret metadata.
    pub fn with_secret_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.secret_metadata.insert(key.into(), value.into());
        self
    }
    
    /// Set operation.
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = operation.into();
        self
    }
}

/// Policy engine for evaluating access to secrets.
#[derive(Debug, Default)]
pub struct PolicyEngine {
    rules: RwLock<HashMap<String, PolicyRule>>,
    secret_rules: RwLock<HashMap<String, Vec<String>>>, // secret_id -> rule_ids
    agent_rules: RwLock<HashMap<u64, Vec<String>>>, // agent_id -> rule_ids
}

impl PolicyEngine {
    /// Create a new policy engine.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a policy rule.
    pub async fn add_rule(&self, rule: PolicyRule) {
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
    }
    
    /// Remove a policy rule.
    pub async fn remove_rule(&self, rule_id: &str) {
        let mut rules = self.rules.write().await;
        rules.remove(rule_id);
        
        // Also remove from indexes
        let mut secret_rules = self.secret_rules.write().await;
        for rules_list in secret_rules.values_mut() {
            rules_list.retain(|id| id != rule_id);
        }
        
        let mut agent_rules = self.agent_rules.write().await;
        for rules_list in agent_rules.values_mut() {
            rules_list.retain(|id| id != rule_id);
        }
    }
    
    /// Associate a rule with a secret.
    pub async fn associate_with_secret(&self, rule_id: &str, secret_id: &str) {
        let mut secret_rules = self.secret_rules.write().await;
        let entry = secret_rules.entry(secret_id.to_string()).or_insert_with(Vec::new);
        if !entry.contains(&rule_id.to_string()) {
            entry.push(rule_id.to_string());
        }
    }
    
    /// Associate a rule with an agent.
    pub async fn associate_with_agent(&self, rule_id: &str, agent_id: u64) {
        let mut agent_rules = self.agent_rules.write().await;
        let entry = agent_rules.entry(agent_id).or_insert_with(Vec::new);
        if !entry.contains(&rule_id.to_string()) {
            entry.push(rule_id.to_string());
        }
    }
    
    /// Evaluate access for a given context.
    pub async fn evaluate(&self, context: &PolicyContext) -> PolicyDecision {
        let rules = self.rules.read().await;
        
        // Collect relevant rules
        let mut relevant_rules = Vec::new();
        
        // Rules associated with this secret
        if let Some(rule_ids) = self.secret_rules.read().await.get(&context.secret_id) {
            for rule_id in rule_ids {
                if let Some(rule) = rules.get(rule_id) {
                    relevant_rules.push(rule);
                }
            }
        }
        
        // Rules associated with this agent
        if let Some(rule_ids) = self.agent_rules.read().await.get(&context.agent_id) {
            for rule_id in rule_ids {
                if let Some(rule) = rules.get(rule_id) {
                    relevant_rules.push(rule);
                }
            }
        }
        
        // Global rules (not associated with specific secret/agent)
        for rule in rules.values() {
            if !relevant_rules.contains(&rule) {
                relevant_rules.push(rule);
            }
        }
        
        // Sort by priority (highest first)
        relevant_rules.sort_by_key(|rule| -rule.priority);
        
        // Evaluate rules in order
        for rule in relevant_rules {
            if let Some(decision) = rule.evaluate(context) {
                return decision;
            }
        }
        
        // Default deny
        PolicyDecision::Deny
    }
    
    /// Check if an agent can read a secret.
    pub async fn can_read(&self, secret_id: &str) -> bool {
        // Simplified check - in real implementation would use proper context
        let context = PolicyContext::new(0, secret_id)
            .with_operation("read");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can write a secret.
    pub async fn can_write(&self, secret_id: &str) -> bool {
        let context = PolicyContext::new(0, secret_id)
            .with_operation("write");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can delete a secret.
    pub async fn can_delete(&self, secret_id: &str) -> bool {
        let context = PolicyContext::new(0, secret_id)
            .with_operation("delete");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can rotate a secret.
    pub async fn can_rotate(&self, secret_id: &str) -> bool {
        let context = PolicyContext::new(0, secret_id)
            .with_operation("rotate");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can list secrets.
    pub async fn can_list(&self) -> bool {
        let context = PolicyContext::new(0, "")
            .with_operation("list");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can manage policies.
    pub async fn can_manage_policies(&self, secret_id: &str) -> bool {
        let context = PolicyContext::new(0, secret_id)
            .with_operation("manage_policies");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can read version history.
    pub async fn can_read_versions(&self, secret_id: &str) -> bool {
        let context = PolicyContext::new(0, secret_id)
            .with_operation("read_versions");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can export secrets.
    pub async fn can_export(&self) -> bool {
        let context = PolicyContext::new(0, "")
            .with_operation("export");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Check if an agent can import secrets.
    pub async fn can_import(&self) -> bool {
        let context = PolicyContext::new(0, "")
            .with_operation("import");
        
        match self.evaluate(&context).await {
            PolicyDecision::Allow | PolicyDecision::AllowWithAudit => true,
            PolicyDecision::Deny => false,
        }
    }
    
    /// Record an access (for auditing and rate limiting).
    pub async fn record_access(&self, secret_id: &str) {
        // In a real implementation, this would update access counts
        // and log to an audit trail
        log::debug!("Access recorded for secret {}", secret_id);
    }
}