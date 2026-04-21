//! Resource monitoring for edge devices.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Resource monitor for edge devices.
pub struct ResourceMonitor {
    collection_interval: Duration,
    alerts: Vec<ResourceAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAlert {
    pub alert_type: AlertType,
    pub threshold: f64,
    pub severity: Severity,
    pub message: String,
    pub triggered_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    CpuHigh,
    MemoryHigh,
    StorageHigh,
    NetworkHigh,
    BatteryLow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl ResourceMonitor {
    /// Create new resource monitor.
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            collection_interval,
            alerts: vec![],
        }
    }

    /// Check resources and generate alerts.
    pub fn check_resources(&mut self, resources: &crate::DeviceResources) -> Vec<ResourceAlert> {
        let mut new_alerts = vec![];

        // CPU alert
        if resources.cpu_percent > 80.0 {
            let severity = if resources.cpu_percent > 95.0 {
                Severity::Critical
            } else if resources.cpu_percent > 90.0 {
                Severity::High
            } else {
                Severity::Medium
            };

            new_alerts.push(ResourceAlert {
                alert_type: AlertType::CpuHigh,
                threshold: 80.0,
                severity,
                message: format!("CPU usage at {:.1}%", resources.cpu_percent),
                triggered_at: Some(chrono::Utc::now().timestamp() as u64),
            });
        }

        // Memory alert
        if resources.memory_percent > 80.0 {
            let severity = if resources.memory_percent > 95.0 {
                Severity::Critical
            } else if resources.memory_percent > 90.0 {
                Severity::High
            } else {
                Severity::Medium
            };

            new_alerts.push(ResourceAlert {
                alert_type: AlertType::MemoryHigh,
                threshold: 80.0,
                severity,
                message: format!("Memory usage at {:.1}%", resources.memory_percent),
                triggered_at: Some(chrono::Utc::now().timestamp() as u64),
            });
        }

        // Storage alert
        let storage_used = (resources.storage_used_mb as f64 / resources.storage_total_mb as f64) * 100.0;
        if storage_used > 80.0 {
            new_alerts.push(ResourceAlert {
                alert_type: AlertType::StorageHigh,
                threshold: 80.0,
                severity: Severity::Medium,
                message: format!("Storage usage at {:.1}%", storage_used),
                triggered_at: Some(chrono::Utc::now().timestamp() as u64),
            });
        }

        // Battery alert
        if let Some(battery) = resources.battery_percent {
            if battery < 20.0 {
                let severity = if battery < 10.0 {
                    Severity::Critical
                } else {
                    Severity::High
                };

                new_alerts.push(ResourceAlert {
                    alert_type: AlertType::BatteryLow,
                    threshold: 20.0,
                    severity,
                    message: format!("Battery level at {:.1}%", battery),
                    triggered_at: Some(chrono::Utc::now().timestamp() as u64),
                });
            }
        }

        // Add new alerts
        self.alerts.extend(new_alerts.clone());

        info!("Resource check complete: {} alerts", new_alerts.len());

        new_alerts
    }

    /// Get all alerts.
    pub fn get_alerts(&self) -> &[ResourceAlert] {
        &self.alerts
    }

    /// Clear alerts older than specified duration.
    pub fn clear_old_alerts(&mut self, max_age_secs: u64) {
        let now = chrono::Utc::now().timestamp() as u64;
        
        self.alerts.retain(|alert| {
            match alert.triggered_at {
                Some(time) => now - time < max_age_secs,
                None => true,
            }
        });
    }

    /// Get alert statistics.
    pub fn get_alert_stats(&self) -> AlertStats {
        let critical = self.alerts.iter()
            .filter(|a| a.severity == Severity::Critical)
            .count();
        let high = self.alerts.iter()
            .filter(|a| a.severity == Severity::High)
            .count();
        let medium = self.alerts.iter()
            .filter(|a| a.severity == Severity::Medium)
            .count();
        let low = self.alerts.iter()
            .filter(|a| a.severity == Severity::Low)
            .count();

        AlertStats {
            total: self.alerts.len() as i64,
            critical: critical as i32,
            high: high as i32,
            medium: medium as i32,
            low: low as i32,
        }
    }

    /// Start monitoring loop.
    pub async fn start_monitoring<F>(&mut self, mut collector: F)
    where
        F: FnMut() -> crate::DeviceResources + Send + 'static,
    {
        let mut interval = tokio::time::interval(self.collection_interval);

        loop {
            interval.tick().await;

            let resources = collector();
            let alerts = self.check_resources(&resources);

            for alert in alerts {
                // Would send alert to notification system
                info!("Alert: {} - {}", alert.alert_type, alert.message);
            }
        }
    }
}

/// Alert statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    pub total: i64,
    pub critical: i32,
    pub high: i32,
    pub medium: i32,
    pub low: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor() {
        let mut monitor = ResourceMonitor::new(Duration::from_secs(5));

        let resources = crate::DeviceResources {
            cpu_percent: 95.0,
            memory_percent: 85.0,
            storage_used_mb: 500,
            storage_total_mb: 1000,
            network_rx_mbps: 0.0,
            network_tx_mbps: 0.0,
            battery_percent: Some(15.0),
        };

        let alerts = monitor.check_resources(&resources);
        
        assert_eq!(alerts.len(), 3); // CPU, Memory, Battery
        
        let stats = monitor.get_alert_stats();
        assert!(stats.critical > 0);
    }
}
