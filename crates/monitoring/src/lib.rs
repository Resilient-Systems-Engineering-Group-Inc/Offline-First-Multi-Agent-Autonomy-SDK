//! Advanced monitoring and alerting system.

pub mod metrics;
pub mod alerting;
pub mod dashboards;

use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use metrics::*;
pub use alerting::*;
pub use dashboards::*;

/// Monitoring configuration.
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub metrics_port: u16,
    pub scrape_interval_secs: u64,
    pub alerting_enabled: bool,
    pub notification_channels: Vec<NotificationChannel>,
    pub retention_days: u32,
}

#[derive(Debug, Clone)]
pub enum NotificationChannel {
    Email { smtp_url: String, recipients: Vec<String> },
    Slack { webhook_url: String, channel: String },
    Webhook { url: String, headers: HashMap<String, String> },
    PagerDuty { integration_key: String },
}

/// Monitoring manager.
pub struct MonitoringManager {
    config: MonitoringConfig,
    metrics_registry: RwLock<prometheus::Registry>,
    alerts: RwLock<Vec<Alert>>,
    last_scrape: RwLock<u64>,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub labels: HashMap<String, String>,
    pub triggered_at: u64,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl MonitoringManager {
    /// Create new monitoring manager.
    pub fn new(config: MonitoringConfig) -> Result<Self> {
        let registry = prometheus::Registry::new();
        
        Ok(Self {
            config,
            metrics_registry: RwLock::new(registry),
            alerts: RwLock::new(Vec::new()),
            last_scrape: RwLock::new(0),
        })
    }

    /// Register metrics.
    pub async fn register_metrics(&self) -> Result<()> {
        let registry = self.metrics_registry.read().await;
        
        // Register counters
        let task_counter = prometheus::CounterOpts::new(
            "sdk_tasks_total",
            "Total number of tasks"
        )?;
        registry.register(Box::new(task_counter))?;

        // Register gauges
        let agent_gauge = prometheus::GaugeOpts::new(
            "sdk_agents_active",
            "Number of active agents"
        )?;
        registry.register(Box::new(agent_gauge))?;

        info!("Metrics registered");
        Ok(())
    }

    /// Scrape metrics.
    pub async fn scrape_metrics(&self) -> Result<String> {
        let registry = self.metrics_registry.read().await;
        let metric_families = registry.gather();
        
        let mut output = String::new();
        for mf in metric_families {
            output.push_str(&format!("# HELP {} {}\n", mf.name, mf.help));
            output.push_str(&format!("# TYPE {} {}\n", mf.name, mf.metric[0].counter.as_ref().map(|_| "counter").unwrap_or("gauge")));
            
            for m in mf.metric {
                if let Some(counter) = m.counter {
                    output.push_str(&format!("{} {}\n", mf.name, counter.value));
                }
            }
        }

        *self.last_scrape.write().await = chrono::Utc::now().timestamp() as u64;
        
        Ok(output)
    }

    /// Create alert.
    pub async fn create_alert(
        &self,
        name: &str,
        severity: AlertSeverity,
        message: &str,
        labels: HashMap<String, String>,
    ) -> Result<String> {
        let alert_id = uuid::Uuid::new_v4().to_string();
        
        let alert = Alert {
            id: alert_id.clone(),
            name: name.to_string(),
            severity,
            message: message.to_string(),
            labels,
            triggered_at: chrono::Utc::now().timestamp() as u64,
            resolved: false,
        };

        let mut alerts = self.alerts.write().await;
        alerts.push(alert);

        // Send notification
        if self.config.alerting_enabled {
            self.send_notification(&alert_id).await?;
        }

        info!("Alert created: {}", alert_id);
        Ok(alert_id)
    }

    /// Send notification.
    async fn send_notification(&self, alert_id: &str) -> Result<()> {
        let alerts = self.alerts.read().await;
        let alert = alerts.iter().find(|a| &a.id == alert_id).unwrap();

        for channel in &self.config.notification_channels {
            match channel {
                NotificationChannel::Email { smtp_url, recipients } => {
                    self.send_email_alert(smtp_url, recipients, alert).await?;
                }
                NotificationChannel::Slack { webhook_url, channel } => {
                    self.send_slack_alert(webhook_url, channel, alert).await?;
                }
                NotificationChannel::Webhook { url, headers } => {
                    self.send_webhook_alert(url, headers, alert).await?;
                }
                NotificationChannel::PagerDuty { integration_key } => {
                    self.send_pagerduty_alert(integration_key, alert).await?;
                }
            }
        }

        Ok(())
    }

    /// Send email alert.
    async fn send_email_alert(
        &self,
        _smtp_url: &str,
        _recipients: &[String],
        alert: &Alert,
    ) -> Result<()> {
        // Implementation would use SMTP
        info!("Email alert sent for: {}", alert.name);
        Ok(())
    }

    /// Send Slack alert.
    async fn send_slack_alert(
        &self,
        webhook_url: &str,
        channel: &str,
        alert: &Alert,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "channel": channel,
            "username": "SDK Monitor",
            "icon_emoji": ":warning:",
            "attachments": [{
                "color": self.get_alert_color(&alert.severity),
                "title": alert.name,
                "text": alert.message,
                "fields": [
                    {"title": "Severity", "value": format!("{:?}", alert.severity), "short": true},
                    {"title": "Time", "value": chrono::Utc::now().to_rfc3339(), "short": true}
                ]
            }]
        });

        let _response = reqwest::Client::new()
            .post(webhook_url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    /// Send webhook alert.
    async fn send_webhook_alert(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        alert: &Alert,
    ) -> Result<()> {
        let mut request = reqwest::Client::new().post(url);
        
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let _response = request
            .json(&alert)
            .send()
            .await?;

        Ok(())
    }

    /// Send PagerDuty alert.
    async fn send_pagerduty_alert(
        &self,
        integration_key: &str,
        alert: &Alert,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "routing_key": integration_key,
            "event_action": "trigger",
            "payload": {
                "summary": alert.name,
                "severity": self.get_pagerduty_severity(&alert.severity),
                "source": "sdk-monitor",
                "custom_details": {
                    "message": alert.message,
                    "labels": alert.labels
                }
            }
        });

        let _response = reqwest::Client::new()
            .post("https://events.pagerduty.com/v2/enqueue")
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    /// Get alert color for Slack.
    fn get_alert_color(&self, severity: &AlertSeverity) -> &str {
        match severity {
            AlertSeverity::Info => "blue",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Error => "danger",
            AlertSeverity::Critical => "#ff0000",
        }
    }

    /// Get PagerDuty severity.
    fn get_pagerduty_severity(&self, severity: &AlertSeverity) -> &str {
        match severity {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Error => "error",
            AlertSeverity::Critical => "critical",
        }
    }

    /// Get active alerts.
    pub async fn get_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.iter().filter(|a| !a.resolved).cloned().collect()
    }

    /// Resolve alert.
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved = true;
            info!("Alert resolved: {}", alert_id);
        }

        Ok(())
    }

    /// Get monitoring statistics.
    pub async fn get_stats(&self) -> MonitoringStats {
        let alerts = self.alerts.read().await;
        let last_scrape = *self.last_scrape.read().await;

        let critical = alerts.iter().filter(|a| a.severity == AlertSeverity::Critical && !a.resolved).count();
        let error = alerts.iter().filter(|a| a.severity == AlertSeverity::Error && !a.resolved).count();
        let warning = alerts.iter().filter(|a| a.severity == AlertSeverity::Warning && !a.resolved).count();

        MonitoringStats {
            total_alerts: alerts.len() as i64,
            active_alerts: alerts.iter().filter(|a| !a.resolved).count() as i64,
            critical_alerts: critical as i32,
            error_alerts: error as i32,
            warning_alerts: warning as i32,
            last_scrape_timestamp: last_scrape,
            notification_channels: self.config.notification_channels.len() as i32,
        }
    }
}

/// Monitoring statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitoringStats {
    pub total_alerts: i64,
    pub active_alerts: i64,
    pub critical_alerts: i32,
    pub error_alerts: i32,
    pub warning_alerts: i32,
    pub last_scrape_timestamp: u64,
    pub notification_channels: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_manager() {
        let config = MonitoringConfig {
            metrics_port: 9090,
            scrape_interval_secs: 15,
            alerting_enabled: false,
            notification_channels: vec![],
            retention_days: 30,
        };

        let manager = MonitoringManager::new(config).unwrap();
        manager.register_metrics().await.unwrap();

        let metrics = manager.scrape_metrics().await.unwrap();
        assert!(!metrics.is_empty());

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_alerts, 0);
    }

    #[tokio::test]
    async fn test_alert_creation() {
        let config = MonitoringConfig {
            metrics_port: 9090,
            scrape_interval_secs: 15,
            alerting_enabled: false,
            notification_channels: vec![],
            retention_days: 30,
        };

        let manager = MonitoringManager::new(config).unwrap();

        let mut labels = HashMap::new();
        labels.insert("service".to_string(), "task-planner".to_string());

        let alert_id = manager
            .create_alert("High CPU Usage", AlertSeverity::Warning, "CPU usage > 80%", labels)
            .await
            .unwrap();

        let alerts = manager.get_alerts().await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].id, alert_id);
    }
}
