//! Rule‑based quality assessment.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{DataQualityError, Result};
use crate::validation::ValidationResult;

/// Quality rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRule {
    /// Unique rule identifier.
    pub id: String,
    /// Human‑readable description.
    pub description: String,
    /// Condition expressed in a simple DSL (e.g., "field.completeness > 90").
    pub condition: String,
    /// Severity if rule fails (0‑1).
    pub severity: f64,
    /// Actions to take when rule fails (e.g., "alert", "quarantine").
    pub actions: Vec<String>,
}

impl QualityRule {
    /// Create a new rule.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        condition: impl Into<String>,
        severity: f64,
        actions: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            condition: condition.into(),
            severity,
            actions,
        }
    }
}

/// Rule evaluation context.
#[derive(Debug, Clone, Default)]
pub struct RuleContext {
    /// Metrics values.
    pub metrics: HashMap<String, f64>,
    /// Validation results.
    pub validation_results: Vec<ValidationResult>,
    /// Anomalies detected.
    pub anomalies: Vec<crate::anomaly::Anomaly>,
    /// Custom fields.
    pub custom: HashMap<String, serde_json::Value>,
}

impl RuleContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a metric.
    pub fn add_metric(&mut self, name: impl Into<String>, value: f64) {
        self.metrics.insert(name.into(), value);
    }

    /// Add validation results.
    pub fn add_validation_results(&mut self, results: Vec<ValidationResult>) {
        self.validation_results.extend(results);
    }

    /// Add anomalies.
    pub fn add_anomalies(&mut self, anomalies: Vec<crate::anomaly::Anomaly>) {
        self.anomalies.extend(anomalies);
    }
}

/// Rule evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluation {
    /// Rule that was evaluated.
    pub rule: QualityRule,
    /// Whether the rule passed.
    pub passed: bool,
    /// Message explaining the result.
    pub message: String,
    /// Timestamp of evaluation.
    pub timestamp: u64,
}

/// Simple rule engine that evaluates rules against a context.
pub struct RuleEngine {
    rules: Vec<QualityRule>,
}

impl RuleEngine {
    /// Create a new rule engine.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: QualityRule) {
        self.rules.push(rule);
    }

    /// Evaluate all rules against a context.
    pub fn evaluate(&self, context: &RuleContext) -> Vec<RuleEvaluation> {
        let mut evaluations = Vec::new();
        for rule in &self.rules {
            let passed = self.evaluate_rule(rule, context);
            evaluations.push(RuleEvaluation {
                rule: rule.clone(),
                passed,
                message: if passed {
                    format!("Rule '{}' passed", rule.id)
                } else {
                    format!("Rule '{}' failed: {}", rule.id, rule.description)
                },
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }
        evaluations
    }

    /// Evaluate a single rule.
    fn evaluate_rule(&self, rule: &QualityRule, context: &RuleContext) -> bool {
        // Simple placeholder implementation.
        // In a real system, you'd parse the condition DSL and evaluate against context.
        // For now, we'll just check if there are any validation failures or anomalies.
        if rule.condition.contains("no_validation_failures") {
            return context.validation_results.iter().all(|r| r.passed);
        }
        if rule.condition.contains("no_anomalies") {
            return context.anomalies.is_empty();
        }
        // Default: pass
        true
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_creation() {
        let rule = QualityRule::new(
            "rule1",
            "Completeness must be > 90%",
            "metrics.completeness > 90",
            0.8,
            vec!["alert".to_string()],
        );
        assert_eq!(rule.id, "rule1");
    }

    #[test]
    fn test_rule_engine() {
        let mut engine = RuleEngine::new();
        engine.add_rule(QualityRule::new(
            "test",
            "No validation failures",
            "no_validation_failures",
            0.5,
            vec![],
        ));
        let mut context = RuleContext::new();
        context.add_validation_results(vec![ValidationResult {
            passed: true,
            error: None,
            rule: None,
        }]);
        let evaluations = engine.evaluate(&context);
        assert_eq!(evaluations.len(), 1);
        assert!(evaluations[0].passed);
    }
}