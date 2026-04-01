//! Health monitoring for agents.

use crate::error::{LifecycleError, Result};
use resource_monitor::ResourceMetrics;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Health status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Agent is healthy and functioning normally.
    Healthy,
    /// Agent has minor issues but is still operational.
    Degraded,
    /// Agent is unhealthy and may need intervention.
    Unhealthy,
    /// Agent health is unknown.
    Unknown,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

use std::fmt;

/// Configuration for health checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks.
    pub check_interval_secs: u64,
    /// CPU usage threshold (percentage) for degraded health.
    pub cpu_threshold_degraded: f32,
    /// CPU usage threshold (percentage) for unhealthy health.
    pub cpu_threshold_unhealthy: f32,
    /// Memory usage threshold (percentage) for degraded health.
    pub memory_threshold_degraded: f32,
    /// Memory usage threshold (percentage) for unhealthy health.
    pub memory_threshold_unhealthy: f32,
    /// Maximum allowed response time for internal checks (milliseconds).
    pub max_response_time_ms: u64,
    /// Number of consecutive failures before marking as unhealthy.
    pub consecutive_failures_threshold: usize,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            cpu_threshold_degraded: 80.0,
            cpu_threshold_unhealthy: 95.0,
            memory_threshold_degraded: 85.0,
            memory_threshold_unhealthy: 95.0,
            max_response_time_ms: 5000,
            consecutive_failures_threshold: 3,
        }
    }
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Overall health status.
    pub status: HealthStatus,
    /// Detailed message about the health check.
    pub message: String,
    /// Timestamp of the check.
    pub timestamp: Instant,
    /// Resource metrics if available.
    pub metrics: Option<ResourceMetrics>,
    /// Individual check results.
    pub checks: Vec<IndividualCheck>,
}

/// Individual health check.
#[derive(Debug, Clone)]
pub struct IndividualCheck {
    /// Name of the check.
    pub name: String,
    /// Status of this specific check.
    pub status: HealthStatus,
    /// Optional message.
    pub message: Option<String>,
    /// Duration of the check.
    pub duration: Duration,
}

/// Health monitor for an agent.
pub struct HealthMonitor {
    config: HealthCheckConfig,
    last_check: Option<Instant>,
    consecutive_failures: usize,
    resource_monitor: Option<resource_monitor::SysinfoMonitor>,
}

impl HealthMonitor {
    /// Create a new health monitor with default configuration.
    pub fn new() -> Self {
        Self {
            config: HealthCheckConfig::default(),
            last_check: None,
            consecutive_failures: 0,
            resource_monitor: None,
        }
    }

    /// Create a health monitor with custom configuration.
    pub fn with_config(config: HealthCheckConfig) -> Self {
        Self {
            config,
            last_check: None,
            consecutive_failures: 0,
            resource_monitor: None,
        }
    }

    /// Set a resource monitor for collecting system metrics.
    pub fn with_resource_monitor(mut self, monitor: resource_monitor::SysinfoMonitor) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }

    /// Perform a health check.
    pub async fn check(&mut self) -> Result<HealthCheckResult> {
        let start_time = Instant::now();
        let mut checks = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        // Check 1: Resource usage
        if let Some(ref mut monitor) = self.resource_monitor {
            let resource_check = self.check_resources(monitor).await;
            overall_status = self.worst_status(overall_status, resource_check.status);
            checks.push(resource_check);
        }

        // Check 2: Connectivity (simplified)
        let connectivity_check = self.check_connectivity().await;
        overall_status = self.worst_status(overall_status, connectivity_check.status);
        checks.push(connectivity_check);

        // Check 3: Internal state
        let state_check = self.check_internal_state().await;
        overall_status = self.worst_status(overall_status, state_check.status);
        checks.push(state_check);

        // Update failure counter
        if overall_status == HealthStatus::Unhealthy {
            self.consecutive_failures += 1;
        } else {
            self.consecutive_failures = 0;
        }

        // Check consecutive failures
        if self.consecutive_failures >= self.config.consecutive_failures_threshold {
            overall_status = HealthStatus::Unhealthy;
        }

        let duration = start_time.elapsed();
        self.last_check = Some(start_time);

        // Collect metrics if available
        let metrics = if let Some(ref mut monitor) = self.resource_monitor {
            match monitor.collect().await {
                Ok(m) => Some(m),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(HealthCheckResult {
            status: overall_status,
            message: format!("Health check completed in {:?}", duration),
            timestamp: start_time,
            metrics,
            checks,
        })
    }

    /// Check resource usage.
    async fn check_resources(
        &self,
        monitor: &mut resource_monitor::SysinfoMonitor,
    ) -> IndividualCheck {
        let start = Instant::now();
        let name = "resource_usage".to_string();

        match monitor.collect().await {
            Ok(metrics) => {
                let cpu_status = if metrics.cpu_percent >= self.config.cpu_threshold_unhealthy {
                    HealthStatus::Unhealthy
                } else if metrics.cpu_percent >= self.config.cpu_threshold_degraded {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                let memory_status = if metrics.memory_percent >= self.config.memory_threshold_unhealthy
                {
                    HealthStatus::Unhealthy
                } else if metrics.memory_percent >= self.config.memory_threshold_degraded {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                let overall_status = self.worst_status(cpu_status, memory_status);
                let message = format!(
                    "CPU: {:.1}%, Memory: {:.1}%",
                    metrics.cpu_percent, metrics.memory_percent
                );

                IndividualCheck {
                    name,
                    status: overall_status,
                    message: Some(message),
                    duration: start.elapsed(),
                }
            }
            Err(e) => IndividualCheck {
                name,
                status: HealthStatus::Unhealthy,
                message: Some(format!("Failed to collect metrics: {}", e)),
                duration: start.elapsed(),
            },
        }
    }

    /// Check connectivity (simplified - always returns healthy in this implementation).
    async fn check_connectivity(&self) -> IndividualCheck {
        let start = Instant::now();
        let name = "connectivity".to_string();

        // In a real implementation, this would check network connectivity
        // to other agents or external services.
        IndividualCheck {
            name,
            status: HealthStatus::Healthy,
            message: Some("Connectivity check passed".to_string()),
            duration: start.elapsed(),
        }
    }

    /// Check internal state.
    async fn check_internal_state(&self) -> IndividualCheck {
        let start = Instant::now();
        let name = "internal_state".to_string();

        // In a real implementation, this would check internal data structures,
        // pending tasks, queue sizes, etc.
        IndividualCheck {
            name,
            status: HealthStatus::Healthy,
            message: Some("Internal state OK".to_string()),
            duration: start.elapsed(),
        }
    }

    /// Get the worst (most severe) of two health statuses.
    fn worst_status(&self, a: HealthStatus, b: HealthStatus) -> HealthStatus {
        use HealthStatus::*;
        match (a, b) {
            (Unhealthy, _) | (_, Unhealthy) => Unhealthy,
            (Degraded, _) | (_, Degraded) => Degraded,
            (Healthy, Healthy) => Healthy,
            _ => Unknown,
        }
    }

    /// Get time since last health check.
    pub fn time_since_last_check(&self) -> Option<Duration> {
        self.last_check.map(|t| t.elapsed())
    }

    /// Get the current consecutive failure count.
    pub fn consecutive_failures(&self) -> usize {
        self.consecutive_failures
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.consecutive_failures(), 0);
        assert!(monitor.time_since_last_check().is_none());
    }

    #[tokio::test]
    async fn test_health_check_without_resource_monitor() {
        let mut monitor = HealthMonitor::new();
        let result = monitor.check().await.unwrap();
        
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.checks.len(), 2); // connectivity + internal state
        assert!(result.metrics.is_none());
    }

    #[test]
    fn test_worst_status() {
        let monitor = HealthMonitor::new();
        
        assert_eq!(
            monitor.worst_status(HealthStatus::Healthy, HealthStatus::Healthy),
            HealthStatus::Healthy
        );
        assert_eq!(
            monitor.worst_status(HealthStatus::Healthy, HealthStatus::Degraded),
            HealthStatus::Degraded
        );
        assert_eq!(
            monitor.worst_status(HealthStatus::Degraded, HealthStatus::Unhealthy),
            HealthStatus::Unhealthy
        );
        assert_eq!(
            monitor.worst_status(HealthStatus::Unknown, HealthStatus::Healthy),
            HealthStatus::Unknown
        );
    }
}