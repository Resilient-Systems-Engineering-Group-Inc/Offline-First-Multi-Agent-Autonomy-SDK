//! Alerting rules and evaluation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

/// Alert rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub expr: String,
    pub for_duration: Duration,
    pub severity: AlertSeverity,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert manager.
pub struct AlertManager {
    rules: Vec<AlertRule>,
    active_alerts: HashMap<String, AlertState>,
}

#[derive(Debug, Clone)]
struct AlertState {
    rule_name: String,
    triggered_at: std::time::Instant,
    value: f64,
    resolved: bool,
}

impl AlertManager {
    /// Create new alert manager.
    pub fn new() -> Self {
        Self {
            rules: vec![],
            active_alerts: HashMap::new(),
        }
    }

    /// Add alert rule.
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
        info!("Alert rule added: {}", rule.name);
    }

    /// Evaluate all rules.
    pub fn evaluate(&mut self, metrics: &HashMap<String, f64>) -> Vec<AlertRule> {
        let mut triggered_alerts = vec![];

        for rule in &self.rules {
            if self.evaluate_rule(rule, metrics) {
                triggered_alerts.push(rule.clone());

                // Update or create alert state
                self.active_alerts.entry(rule.name.clone()).or_insert_with(|| {
                    AlertState {
                        rule_name: rule.name.clone(),
                        triggered_at: std::time::Instant::now(),
                        value: 0.0,
                        resolved: false,
                    }
                });
            }
        }

        triggered_alerts
    }

    /// Evaluate single rule.
    fn evaluate_rule(&self, rule: &AlertRule, metrics: &HashMap<String, f64>) -> bool {
        // Simplified evaluation - in production would use expression parser
        if rule.expr.contains(">") {
            let parts: Vec<&str> = rule.expr.split('>').collect();
            if parts.len() == 2 {
                let metric_name = parts[0].trim();
                if let Ok(threshold) = parts[1].trim().parse::<f64>() {
                    if let Some(&value) = metrics.get(metric_name) {
                        return value > threshold;
                    }
                }
            }
        }

        false
    }

    /// Get active alerts.
    pub fn get_active_alerts(&self) -> Vec<&AlertRule> {
        self.rules
            .iter()
            .filter(|rule| self.active_alerts.contains_key(&rule.name))
            .collect()
    }

    /// Resolve alert.
    pub fn resolve_alert(&mut self, rule_name: &str) {
        self.active_alerts.remove(rule_name);
        info!("Alert resolved: {}", rule_name);
    }

    /// Get default alert rules.
    pub fn get_default_rules() -> Vec<AlertRule> {
        let mut rules = vec![];

        // High CPU usage
        rules.push(AlertRule {
            name: "HighCPUUsage".to_string(),
            expr: "sdk_cpu_usage_percent > 80".to_string(),
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            labels: [("severity".to_string(), "warning".to_string())].iter().cloned().collect(),
            annotations: [
                ("summary".to_string(), "High CPU usage detected".to_string()),
                ("description".to_string(), "CPU usage is above 80% for more than 5 minutes".to_string()),
            ].iter().cloned().collect(),
        });

        // High memory usage
        rules.push(AlertRule {
            name: "HighMemoryUsage".to_string(),
            expr: "sdk_memory_usage_bytes > 1073741824".to_string(), // 1GB
            for_duration: Duration::from_secs(300),
            severity: AlertSeverity::Warning,
            labels: [("severity".to_string(), "warning".to_string())].iter().cloned().collect(),
            annotations: [
                ("summary".to_string(), "High memory usage detected".to_string()),
                ("description".to_string(), "Memory usage is above 1GB for more than 5 minutes".to_string()),
            ].iter().cloned().collect(),
        });

        // Task failure rate
        rules.push(AlertRule {
            name: "HighTaskFailureRate".to_string(),
            expr: "sdk_tasks_failed_total > 10".to_string(),
            for_duration: Duration::from_secs(600),
            severity: AlertSeverity::Error,
            labels: [("severity".to_string(), "error".to_string())].iter().cloned().collect(),
            annotations: [
                ("summary".to_string(), "High task failure rate".to_string()),
                ("description".to_string(), "More than 10 tasks have failed in the last 10 minutes".to_string()),
            ].iter().cloned().collect(),
        });

        // Agent offline
        rules.push(AlertRule {
            name: "AgentOffline".to_string(),
            expr: "sdk_agents_active < 1".to_string(),
            for_duration: Duration::from_secs(60),
            severity: AlertSeverity::Critical,
            labels: [("severity".to_string(), "critical".to_string())].iter().cloned().collect(),
            annotations: [
                ("summary".to_string(), "All agents offline".to_string()),
                ("description".to_string(), "No active agents for more than 1 minute".to_string()),
            ].iter().cloned().collect(),
        });

        // High latency
        rules.push(AlertRule {
            name: "HighLatency".to_string(),
            expr: "sdk_network_latency_seconds > 0.5".to_string(),
            for_duration: Duration::from_secs(120),
            severity: AlertSeverity::Warning,
            labels: [("severity".to_string(), "warning".to_string())].iter().cloned().collect(),
            annotations: [
                ("summary".to_string(), "High network latency".to_string()),
                ("description".to_string(), "Network latency is above 500ms for more than 2 minutes".to_string()),
            ].iter().cloned().collect(),
        });

        rules
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        let mut manager = Self::new();
        
        // Load default rules
        for rule in AlertManager::get_default_rules() {
            manager.add_rule(rule);
        }

        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_manager() {
        let mut manager = AlertManager::new();

        // Add custom rule
        let rule = AlertRule {
            name: "CustomAlert".to_string(),
            expr: "custom_metric > 100".to_string(),
            for_duration: Duration::from_secs(60),
            severity: AlertSeverity::Info,
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        manager.add_rule(rule);

        // Evaluate with metrics
        let mut metrics = HashMap::new();
        metrics.insert("sdk_cpu_usage_percent".to_string(), 85.0);
        metrics.insert("custom_metric".to_string(), 150.0);

        let alerts = manager.evaluate(&metrics);
        assert_eq!(alerts.len(), 2);
    }

    #[test]
    fn test_default_rules() {
        let rules = AlertManager::get_default_rules();
        assert_eq!(rules.len(), 5);

        assert!(rules.iter().any(|r| r.name == "HighCPUUsage"));
        assert!(rules.iter().any(|r| r.name == "HighMemoryUsage"));
        assert!(rules.iter().any(|r| r.name == "HighTaskFailureRate"));
        assert!(rules.iter().any(|r| r.name == "AgentOffline"));
        assert!(rules.iter().any(|r| r.name == "HighLatency"));
    }
}
