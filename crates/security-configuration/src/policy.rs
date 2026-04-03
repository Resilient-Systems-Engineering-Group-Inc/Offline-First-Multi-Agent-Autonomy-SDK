//! Policy evaluation engine.

use crate::config::{Policy, PolicyRule};
use crate::error::{Result, SecurityConfigError};
use std::collections::HashMap;

/// Context for evaluating a policy rule.
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    /// The agent ID performing the action.
    pub agent_id: Option<String>,
    /// The action being performed.
    pub action: String,
    /// The resource being accessed.
    pub resource: String,
    /// Additional key‑value parameters.
    pub params: HashMap<String, String>,
    /// Current capabilities of the agent.
    pub capabilities: HashMap<String, u8>,
    /// Network context (IP, port, etc.)
    pub network: Option<NetworkContext>,
}

/// Network‑specific context.
#[derive(Debug, Clone)]
pub struct NetworkContext {
    /// Source IP address.
    pub source_ip: String,
    /// Destination IP address.
    pub dest_ip: String,
    /// Destination port.
    pub dest_port: u16,
    /// Whether TLS is used.
    pub tls: bool,
}

/// Result of evaluating a single rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleDecision {
    /// The rule explicitly allows the action.
    Allow,
    /// The rule explicitly denies the action.
    Deny,
    /// The rule does not apply (neutral).
    NotApplicable,
}

/// Result of evaluating a whole policy.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// The policy ID.
    pub policy_id: String,
    /// Overall decision (allow/deny).
    pub decision: RuleDecision,
    /// Which rule caused the decision.
    pub matched_rule: Option<String>,
    /// Human‑readable explanation.
    pub explanation: String,
}

/// Evaluates a single rule against the given context.
pub fn evaluate_rule(rule: &PolicyRule, ctx: &EvaluationContext) -> RuleDecision {
    match rule {
        PolicyRule::Action {
            action,
            resource,
            allow,
            conditions,
        } => {
            if !action_matches(action, &ctx.action) {
                return RuleDecision::NotApplicable;
            }
            if !resource_matches(resource, &ctx.resource) {
                return RuleDecision::NotApplicable;
            }
            if !conditions_match(conditions, &ctx.params) {
                return RuleDecision::NotApplicable;
            }
            if *allow {
                RuleDecision::Allow
            } else {
                RuleDecision::Deny
            }
        }
        PolicyRule::Capability { capability, level } => {
            match ctx.capabilities.get(capability) {
                Some(actual) if *actual >= *level => RuleDecision::Allow,
                Some(_) => RuleDecision::Deny,
                None => RuleDecision::NotApplicable,
            }
        }
        PolicyRule::Crypto {
            algorithm,
            min_key_length,
        } => {
            // For simplicity, we assume the context includes crypto parameters.
            // In a real implementation you would check the actual algorithm and key length.
            if ctx.params.get("crypto_algorithm") == Some(algorithm)
                && ctx
                    .params
                    .get("key_length")
                    .and_then(|s| s.parse::<u32>().ok())
                    .map(|len| len >= *min_key_length)
                    .unwrap_or(false)
            {
                RuleDecision::Allow
            } else {
                RuleDecision::Deny
            }
        }
        PolicyRule::Network {
            allowed_ips,
            allowed_ports,
            require_tls,
        } => {
            let network = match &ctx.network {
                Some(n) => n,
                None => return RuleDecision::NotApplicable,
            };
            let ip_allowed = allowed_ips
                .iter()
                .any(|cidr| ip_matches_cidr(&network.dest_ip, cidr));
            let port_allowed = allowed_ports.contains(&network.dest_port);
            let tls_ok = !*require_tls || network.tls;
            if ip_allowed && port_allowed && tls_ok {
                RuleDecision::Allow
            } else {
                RuleDecision::Deny
            }
        }
        PolicyRule::Custom { subtype, params } => {
            // Custom rules are not evaluated by default; they are considered NotApplicable.
            // Extensions can override this behavior.
            RuleDecision::NotApplicable
        }
    }
}

/// Evaluates a whole policy (all its rules) using the given context.
///
/// The policy's decision is determined by the first rule that yields Allow or Deny.
/// If no rule applies, the policy is considered NotApplicable.
pub fn evaluate_policy(policy: &Policy, ctx: &EvaluationContext) -> PolicyDecision {
    for rule in &policy.rules {
        let decision = evaluate_rule(rule, ctx);
        match decision {
            RuleDecision::Allow => {
                return PolicyDecision {
                    policy_id: policy.id.clone(),
                    decision: RuleDecision::Allow,
                    matched_rule: Some(format!("{:?}", rule)),
                    explanation: format!("Policy '{}' allows the action", policy.name),
                };
            }
            RuleDecision::Deny => {
                return PolicyDecision {
                    policy_id: policy.id.clone(),
                    decision: RuleDecision::Deny,
                    matched_rule: Some(format!("{:?}", rule)),
                    explanation: format!("Policy '{}' denies the action", policy.name),
                };
            }
            RuleDecision::NotApplicable => continue,
        }
    }
    PolicyDecision {
        policy_id: policy.id.clone(),
        decision: RuleDecision::NotApplicable,
        matched_rule: None,
        explanation: format!("Policy '{}' does not apply", policy.name),
    }
}

/// Evaluates a list of policies and returns the final decision.
///
/// The evaluation follows the "first‑applicable" order: policies are evaluated in the given order,
/// and the first policy that yields Allow or Deny determines the outcome.
/// If no policy applies, the result is NotApplicable.
pub fn evaluate_policies(
    policies: &[&Policy],
    ctx: &EvaluationContext,
) -> (RuleDecision, Vec<PolicyDecision>) {
    let mut decisions = Vec::new();
    for policy in policies {
        let decision = evaluate_policy(policy, ctx);
        match decision.decision {
            RuleDecision::Allow | RuleDecision::Deny => {
                decisions.push(decision);
                return (decision.decision, decisions);
            }
            RuleDecision::NotApplicable => {
                decisions.push(decision);
            }
        }
    }
    (RuleDecision::NotApplicable, decisions)
}

// --- Helper functions (simplified) ---

fn action_matches(pattern: &str, action: &str) -> bool {
    pattern == "*" || pattern == action
}

fn resource_matches(pattern: &str, resource: &str) -> bool {
    pattern == "*" || pattern == resource
}

fn conditions_match(conditions: &HashMap<String, String>, params: &HashMap<String, String>) -> bool {
    conditions
        .iter()
        .all(|(key, expected)| params.get(key).map(|v| v == expected).unwrap_or(false))
}

fn ip_matches_cidr(_ip: &str, _cidr: &str) -> bool {
    // Simplified: always true for demonstration.
    // In reality you would use a proper CIDR matching library.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PolicyRule;

    fn sample_policy(id: &str) -> Policy {
        Policy {
            id: id.to_string(),
            name: format!("Policy {}", id),
            description: "Test".to_string(),
            rules: vec![
                PolicyRule::Action {
                    action: "read".to_string(),
                    resource: "file:*".to_string(),
                    allow: true,
                    conditions: HashMap::new(),
                },
                PolicyRule::Action {
                    action: "write".to_string(),
                    resource: "file:*".to_string(),
                    allow: false,
                    conditions: HashMap::new(),
                },
            ],
            mandatory: false,
            tags: vec![],
        }
    }

    #[test]
    fn test_evaluate_rule() {
        let rule = PolicyRule::Action {
            action: "read".to_string(),
            resource: "file:*".to_string(),
            allow: true,
            conditions: HashMap::new(),
        };
        let ctx = EvaluationContext {
            action: "read".to_string(),
            resource: "file:test.txt".to_string(),
            ..Default::default()
        };
        assert_eq!(evaluate_rule(&rule, &ctx), RuleDecision::Allow);

        let ctx2 = EvaluationContext {
            action: "write".to_string(),
            resource: "file:test.txt".to_string(),
            ..Default::default()
        };
        assert_eq!(evaluate_rule(&rule, &ctx2), RuleDecision::NotApplicable);
    }

    #[test]
    fn test_evaluate_policy() {
        let policy = sample_policy("test");
        let ctx = EvaluationContext {
            action: "read".to_string(),
            resource: "file:test.txt".to_string(),
            ..Default::default()
        };
        let decision = evaluate_policy(&policy, &ctx);
        assert_eq!(decision.decision, RuleDecision::Allow);
        assert!(decision.matched_rule.is_some());

        let ctx2 = EvaluationContext {
            action: "write".to_string(),
            resource: "file:test.txt".to_string(),
            ..Default::default()
        };
        let decision2 = evaluate_policy(&policy, &ctx2);
        assert_eq!(decision2.decision, RuleDecision::Deny);
    }

    #[test]
    fn test_evaluate_policies() {
        let policy1 = sample_policy("p1");
        let policy2 = sample_policy("p2");
        let policies = vec![&policy1, &policy2];
        let ctx = EvaluationContext {
            action: "read".to_string(),
            resource: "file:test.txt".to_string(),
            ..Default::default()
        };
        let (final_decision, individual) = evaluate_policies(&policies, &ctx);
        assert_eq!(final_decision, RuleDecision::Allow);
        assert_eq!(individual.len(), 1); // only first policy applied
    }
}