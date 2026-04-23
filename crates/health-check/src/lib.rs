//! Health check system for the Multi-Agent SDK.
//!
//! Provides:
//! - Service health monitoring
//! - Dependency health checks (database, cache, message queue)
//! - Automated recovery
//! - Health status aggregation

pub mod checker;
pub mod aggregator;
pub mod recovery;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use checker::*;
pub use aggregator::*;
pub use recovery::*;

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub check_interval_secs: u64,
    pub timeout_secs: u64,
    pub failure_threshold: u32,
    pub recovery_threshold: u32,
    pub services: Vec<ServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub health_endpoint: String,
    pub critical: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            timeout_secs: 5,
            failure_threshold: 3,
            recovery_threshold: 2,
            services: vec![
                ServiceConfig {
                    name: "database".to_string(),
                    health_endpoint: "http://localhost:5432/health".to_string(),
                    critical: true,
                },
                ServiceConfig {
                    name: "redis".to_string(),
                    health_endpoint: "http://localhost:6379/health".to_string(),
                    critical: false,
                },
            ],
        }
    }
}

/// Health check manager.
pub struct HealthCheckManager {
    config: HealthCheckConfig,
    service_status: RwLock<HashMap<String, ServiceHealthStatus>>,
    running: RwLock<bool>,
}

impl HealthCheckManager {
    /// Create new health check manager.
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            service_status: RwLock::new(HashMap::new()),
            running: RwLock::new(false),
        }
    }

    /// Start health checks.
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;

        let check_interval = self.config.check_interval_secs;
        let services = self.config.services.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(check_interval));

            loop {
                interval.tick().await;

                for service in &services {
                    let status = check_service_health(service).await;
                    
                    let mut status_map = Self::get_service_status_lock().await;
                    status_map.insert(service.name.clone(), status);
                }
            }
        });

        info!("Health checks started");
        Ok(())
    }

    /// Stop health checks.
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Health checks stopped");
        Ok(())
    }

    /// Get overall health status.
    pub async fn get_health_status(&self) -> HealthStatus {
        let status = self.service_status.read().await;

        let all_healthy = status.values().all(|s| s.status == HealthStatusEnum::Healthy);
        let any_critical_down = status.values()
            .any(|s| s.status == HealthStatusEnum::Unhealthy && s.critical);

        if all_healthy {
            HealthStatus::Healthy
        } else if any_critical_down {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        }
    }

    /// Get service status.
    pub async fn get_service_status(&self, service_name: &str) -> Option<ServiceHealthStatus> {
        let status = self.service_status.read().await;
        status.get(service_name).cloned()
    }

    /// Get all service statuses.
    pub async fn get_all_statuses(&self) -> HashMap<String, ServiceHealthStatus> {
        let status = self.service_status.read().await;
        status.clone()
    }

    /// Manual health check for service.
    pub async fn check_service(&self, service_name: &str) -> Result<ServiceHealthStatus> {
        let service = self.config.services.iter()
            .find(|s| s.name == service_name)
            .ok_or_else(|| anyhow::anyhow!("Service not found: {}", service_name))?;

        let status = check_service_health(service).await;
        
        self.service_status.write().await
            .insert(service_name.to_string(), status.clone());

        Ok(status)
    }

    /// Get health report.
    pub async fn get_health_report(&self) -> HealthReport {
        let status = self.service_status.read().await;
        let overall = self.get_health_status().await;

        let healthy_count = status.values()
            .filter(|s| s.status == HealthStatusEnum::Healthy)
            .count();

        let unhealthy_count = status.values()
            .filter(|s| s.status == HealthStatusEnum::Unhealthy)
            .count();

        HealthReport {
            timestamp: chrono::Utc::now(),
            overall_status: overall,
            total_services: status.len() as i32,
            healthy_services: healthy_count as i32,
            unhealthy_services: unhealthy_count as i32,
            services: status.clone(),
        }
    }
}

/// Check service health.
async fn check_service_health(service: &ServiceConfig) -> ServiceHealthStatus {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    let status = match client.get(&service.health_endpoint).send().await {
        Ok(response) => {
            if response.status().is_success() {
                HealthStatusEnum::Healthy
            } else {
                HealthStatusEnum::Degraded
            }
        }
        Err(_) => HealthStatusEnum::Unhealthy,
    };

    ServiceHealthStatus {
        name: service.name.clone(),
        status,
        last_check: chrono::Utc::now(),
        consecutive_failures: 0,
        critical: service.critical,
    }
}

/// Health status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatusEnum {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Service health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    pub name: String,
    pub status: HealthStatusEnum,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub consecutive_failures: u32,
    pub critical: bool,
}

/// Overall health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub overall_status: HealthStatus,
    pub total_services: i32,
    pub healthy_services: i32,
    pub unhealthy_services: i32,
    pub services: HashMap<String, ServiceHealthStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_manager() {
        let config = HealthCheckConfig::default();
        let manager = HealthCheckManager::new(config);

        // Get health status
        let status = manager.get_health_status().await;
        assert!(matches!(status, HealthStatus::Healthy | HealthStatus::Degraded));

        // Get health report
        let report = manager.get_health_report().await;
        assert!(!report.timestamp.is_min());
    }
}
