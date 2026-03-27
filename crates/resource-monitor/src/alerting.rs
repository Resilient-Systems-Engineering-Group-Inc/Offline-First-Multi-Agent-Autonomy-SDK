//! Alerting based on resource thresholds.

use crate::ResourceMetrics;
use common::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Severity of an alert.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// An alert triggered by a resource threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub timestamp: u64, // Unix timestamp in seconds
    pub resource: String,
    pub value: f32,
    pub threshold: f32,
}

/// Configuration for a threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Resource identifier (e.g., "cpu_usage", "battery_level").
    pub resource: String,
    /// Threshold value.
    pub threshold: f32,
    /// Comparison operator: "gt" (greater than), "lt" (less than), "eq".
    pub operator: String,
    /// Severity if threshold is breached.
    pub severity: AlertSeverity,
    /// Cooldown period in seconds before sending another alert for the same resource.
    pub cooldown_secs: u64,
}

/// Alert manager that evaluates metrics against thresholds.
pub struct AlertManager {
    thresholds: Vec<ThresholdConfig>,
    last_alert_time: HashMap<String, u64>, // resource -> timestamp
    alert_tx: Option<mpsc::Sender<Alert>>,
}

impl AlertManager {
    /// Create a new alert manager with given thresholds.
    pub fn new(thresholds: Vec<ThresholdConfig>) -> Self {
        Self {
            thresholds,
            last_alert_time: HashMap::new(),
            alert_tx: None,
        }
    }

    /// Set the alert channel for sending alerts.
    pub fn set_alert_channel(&mut self, tx: mpsc::Sender<Alert>) {
        self.alert_tx = Some(tx);
    }

    /// Evaluate a single metric and produce alerts if needed.
    pub async fn evaluate(&mut self, metrics: &ResourceMetrics, timestamp: u64) -> Result<Vec<Alert>> {
        let mut alerts = Vec::new();

        for threshold in &self.thresholds {
            let value = match threshold.resource.as_str() {
                "cpu_usage" => metrics.cpu_usage,
                "memory_usage" => (metrics.memory_used as f32) / (metrics.memory_total as f32) * 100.0,
                "battery_level" => metrics.battery_level.unwrap_or(100.0),
                "network_tx" => metrics.network_tx as f32,
                "network_rx" => metrics.network_rx as f32,
                "disk_usage" => (metrics.disk_used as f32) / (metrics.disk_total as f32) * 100.0,
                _ => continue,
            };

            let breached = match threshold.operator.as_str() {
                "gt" => value > threshold.threshold,
                "lt" => value < threshold.threshold,
                "eq" => (value - threshold.threshold).abs() < 0.001,
                _ => false,
            };

            if breached {
                // Check cooldown
                if let Some(last_time) = self.last_alert_time.get(&threshold.resource) {
                    if timestamp - last_time < threshold.cooldown_secs {
                        continue;
                    }
                }

                let alert = Alert {
                    id: format!("{}_{}", threshold.resource, timestamp),
                    title: format!("{} threshold breached", threshold.resource),
                    description: format!("{} is {} (threshold {})", threshold.resource, value, threshold.threshold),
                    severity: threshold.severity.clone(),
                    timestamp,
                    resource: threshold.resource.clone(),
                    value,
                    threshold: threshold.threshold,
                };

                alerts.push(alert.clone());
                self.last_alert_time.insert(threshold.resource.clone(), timestamp);

                // Send via channel if available
                if let Some(tx) = &self.alert_tx {
                    let _ = tx.send(alert).await;
                }
            }
        }

        Ok(alerts)
    }

    /// Clear cooldown for a resource.
    pub fn clear_cooldown(&mut self, resource: &str) {
        self.last_alert_time.remove(resource);
    }
}

/// Example threshold configurations.
pub fn default_thresholds() -> Vec<ThresholdConfig> {
    vec![
        ThresholdConfig {
            resource: "cpu_usage".to_string(),
            threshold: 90.0,
            operator: "gt".to_string(),
            severity: AlertSeverity::Critical,
            cooldown_secs: 300, // 5 minutes
        },
        ThresholdConfig {
            resource: "battery_level".to_string(),
            threshold: 20.0,
            operator: "lt".to_string(),
            severity: AlertSeverity::Warning,
            cooldown_secs: 600, // 10 minutes
        },
        ThresholdConfig {
            resource: "memory_usage".to_string(),
            threshold: 85.0,
            operator: "gt".to_string(),
            severity: AlertSeverity::Warning,
            cooldown_secs: 300,
        },
    ]
}