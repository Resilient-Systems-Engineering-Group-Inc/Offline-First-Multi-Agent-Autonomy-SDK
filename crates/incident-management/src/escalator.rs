//! Escalation policies for incidents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{IncidentError, Result};
use crate::model::{Incident, IncidentSeverity, IncidentStatus};

/// Escalation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    /// Severity threshold.
    pub severity: IncidentSeverity,
    /// Time threshold in minutes before escalating.
    pub time_threshold_minutes: u32,
    /// Target status to escalate to.
    pub target_status: IncidentStatus,
    /// Message to include.
    pub message: String,
    /// Whether to notify additional parties.
    pub notify: bool,
}

/// Escalation policy consisting of multiple rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    /// Policy name.
    pub name: String,
    /// Rules sorted by priority.
    pub rules: Vec<EscalationRule>,
}

impl EscalationPolicy {
    /// Create a new policy.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rules: Vec::new(),
        }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: EscalationRule) {
        self.rules.push(rule);
    }

    /// Evaluate an incident against the policy.
    /// Returns a list of actions to take (escalations).
    pub fn evaluate(&self, incident: &Incident, now: DateTime<Utc>) -> Vec<EscalationAction> {
        let mut actions = Vec::new();
        for rule in &self.rules {
            if incident.severity < rule.severity {
                continue;
            }
            let age_minutes = (now - incident.detected_at).num_minutes() as u32;
            if age_minutes >= rule.time_threshold_minutes {
                actions.push(EscalationAction {
                    rule: rule.clone(),
                    incident_id: incident.id,
                    timestamp: now,
                });
            }
        }
        actions
    }
}

/// An escalation action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationAction {
    /// Rule that triggered the escalation.
    pub rule: EscalationRule,
    /// Incident ID.
    pub incident_id: crate::model::IncidentId,
    /// Timestamp of escalation.
    pub timestamp: DateTime<Utc>,
}

/// Escalation engine that applies policies.
pub struct EscalationEngine {
    policies: Vec<EscalationPolicy>,
}

impl EscalationEngine {
    /// Create a new escalation engine.
    pub fn new() -> Self {
        Self { policies: Vec::new() }
    }

    /// Add a policy.
    pub fn add_policy(&mut self, policy: EscalationPolicy) {
        self.policies.push(policy);
    }

    /// Evaluate all policies for a given incident.
    pub fn evaluate_incident(&self, incident: &Incident) -> Vec<EscalationAction> {
        let now = Utc::now();
        let mut actions = Vec::new();
        for policy in &self.policies {
            actions.extend(policy.evaluate(incident, now));
        }
        actions
    }

    /// Apply escalation actions to an incident (update status, notify, etc.).
    pub async fn apply_actions(
        &self,
        actions: Vec<EscalationAction>,
        tracker: &crate::tracker::IncidentTracker,
    ) -> Result<()> {
        for action in actions {
            // Update incident status to target status
            tracker.update_status(action.incident_id, action.rule.target_status).await?;
            // In a real implementation, send notifications, etc.
            tracing::info!(
                "Incident {} escalated via rule '{}': {}",
                action.incident_id,
                action.rule.message,
                action.rule.target_status
            );
        }
        Ok(())
    }
}

impl Default for EscalationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{IncidentSource, IncidentSeverity, IncidentStatus};

    #[test]
    fn test_escalation_rule_evaluation() {
        let mut policy = EscalationPolicy::new("test");
        policy.add_rule(EscalationRule {
            severity: IncidentSeverity::Warning,
            time_threshold_minutes: 5,
            target_status: IncidentStatus::Acknowledged,
            message: "Escalate after 5 minutes".to_string(),
            notify: true,
        });

        let incident = Incident::new(
            "test",
            "test",
            IncidentSeverity::Warning,
            IncidentSource::Custom("test".to_string()),
        );
        // Simulate incident detected 10 minutes ago
        let mut incident = incident;
        incident.detected_at = Utc::now() - chrono::Duration::minutes(10);
        let actions = policy.evaluate(&incident, Utc::now());
        assert!(!actions.is_empty());
    }
}