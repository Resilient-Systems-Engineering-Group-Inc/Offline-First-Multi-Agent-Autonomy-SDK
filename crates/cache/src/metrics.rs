//! Cache metrics and monitoring.

use serde::{Deserialize, Serialize};

/// Cache metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheMetrics {
    pub hits: i64,
    pub misses: i64,
    pub sets: i64,
    pub deletes: i64,
    pub errors: i64,
    pub total_get_latency_ms: f64,
    pub total_set_latency_ms: f64,
    pub get_count: i64,
    pub set_count: i64,
    pub avg_get_latency_ms: f64,
    pub avg_set_latency_ms: f64,
}

impl CacheMetrics {
    pub fn update_get_latency(&mut self, latency_ms: f64) {
        self.total_get_latency_ms += latency_ms;
        self.get_count += 1;
        self.avg_get_latency_ms = self.total_get_latency_ms / self.get_count as f64;
    }

    pub fn update_set_latency(&mut self, latency_ms: f64) {
        self.total_set_latency_ms += latency_ms;
        self.set_count += 1;
        self.avg_set_latency_ms = self.total_set_latency_ms / self.set_count as f64;
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    pub fn operations_per_second(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            (self.hits + self.misses + self.sets + self.deletes) as f64 / duration_secs
        } else {
            0.0
        }
    }
}

/// Cache metrics snapshot for reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub hits: i64,
    pub misses: i64,
    pub hit_rate: f64,
    pub avg_get_latency_ms: f64,
    pub avg_set_latency_ms: f64,
    pub total_operations: i64,
}

impl CacheMetricsSnapshot {
    pub fn from_metrics(metrics: &CacheMetrics) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            hits: metrics.hits,
            misses: metrics.misses,
            hit_rate: metrics.hit_rate(),
            avg_get_latency_ms: metrics.avg_get_latency_ms,
            avg_set_latency_ms: metrics.avg_set_latency_ms,
            total_operations: metrics.hits + metrics.misses + metrics.sets + metrics.deletes,
        }
    }
}

/// Cache alert thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheAlertThresholds {
    pub min_hit_rate: f64,
    pub max_latency_ms: f64,
    pub max_memory_usage_percent: f64,
    pub max_error_rate: f64,
}

impl Default for CacheAlertThresholds {
    fn default() -> Self {
        Self {
            min_hit_rate: 0.5,
            max_latency_ms: 100.0,
            max_memory_usage_percent: 90.0,
            max_error_rate: 0.01,
        }
    }
}

/// Cache alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheAlert {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub current_value: f64,
    pub threshold: f64,
}

/// Cache health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Cache health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealthCheck {
    pub status: CacheHealth,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checks: Vec<HealthCheckItem>,
    pub alerts: Vec<CacheAlert>,
}

/// Health check item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckItem {
    pub name: String,
    pub status: String,
    pub message: String,
    pub value: Option<f64>,
}

/// Cache health checker.
pub struct CacheHealthChecker {
    thresholds: CacheAlertThresholds,
}

impl CacheHealthChecker {
    pub fn new(thresholds: CacheAlertThresholds) -> Self {
        Self { thresholds }
    }

    pub fn check(&self, metrics: &CacheMetrics) -> CacheHealthCheck {
        let mut checks = Vec::new();
        let mut alerts = Vec::new();
        let mut status = CacheHealth::Healthy;

        // Check hit rate
        let hit_rate = metrics.hit_rate();
        checks.push(HealthCheckItem {
            name: "hit_rate".to_string(),
            status: if hit_rate >= self.thresholds.min_hit_rate { "ok".to_string() } else { "warning".to_string() },
            message: format!("Hit rate: {:.2}%", hit_rate * 100.0),
            value: Some(hit_rate),
        });

        if hit_rate < self.thresholds.min_hit_rate {
            alerts.push(CacheAlert {
                timestamp: chrono::Utc::now(),
                alert_type: "low_hit_rate".to_string(),
                severity: "warning".to_string(),
                message: format!("Cache hit rate {:.2}% is below threshold {:.2}%", 
                    hit_rate * 100.0, self.thresholds.min_hit_rate * 100.0),
                current_value: hit_rate,
                threshold: self.thresholds.min_hit_rate,
            });
            status = CacheHealth::Degraded;
        }

        // Check latency
        checks.push(HealthCheckItem {
            name: "get_latency".to_string(),
            status: if metrics.avg_get_latency_ms <= self.thresholds.max_latency_ms { "ok".to_string() } else { "warning".to_string() },
            message: format!("Avg GET latency: {:.2}ms", metrics.avg_get_latency_ms),
            value: Some(metrics.avg_get_latency_ms),
        });

        if metrics.avg_get_latency_ms > self.thresholds.max_latency_ms {
            alerts.push(CacheAlert {
                timestamp: chrono::Utc::now(),
                alert_type: "high_latency".to_string(),
                severity: "warning".to_string(),
                message: format!("Cache latency {:.2}ms exceeds threshold {:.2}ms", 
                    metrics.avg_get_latency_ms, self.thresholds.max_latency_ms),
                current_value: metrics.avg_get_latency_ms,
                threshold: self.thresholds.max_latency_ms,
            });
            status = CacheHealth::Degraded;
        }

        // Check error rate
        let total_ops = metrics.hits + metrics.misses + metrics.sets + metrics.deletes;
        let error_rate = if total_ops > 0 {
            metrics.errors as f64 / total_ops as f64
        } else {
            0.0
        };

        checks.push(HealthCheckItem {
            name: "error_rate".to_string(),
            status: if error_rate <= self.thresholds.max_error_rate { "ok".to_string() } else { "error".to_string() },
            message: format!("Error rate: {:.2}%", error_rate * 100.0),
            value: Some(error_rate),
        });

        if error_rate > self.thresholds.max_error_rate {
            alerts.push(CacheAlert {
                timestamp: chrono::Utc::now(),
                alert_type: "high_error_rate".to_string(),
                severity: "error".to_string(),
                message: format!("Cache error rate {:.2}% exceeds threshold {:.2}%", 
                    error_rate * 100.0, self.thresholds.max_error_rate * 100.0),
                current_value: error_rate,
                threshold: self.thresholds.max_error_rate,
            });
            status = CacheHealth::Unhealthy;
        }

        CacheHealthCheck {
            status,
            timestamp: chrono::Utc::now(),
            checks,
            alerts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_metrics() {
        let mut metrics = CacheMetrics::default();

        metrics.hits = 80;
        metrics.misses = 20;
        metrics.sets = 50;
        metrics.deletes = 10;

        assert_eq!(metrics.hit_rate(), 0.8);
        assert_eq!(metrics.miss_rate(), 0.2);
        assert_eq!(metrics.operations_per_second(10.0), 16.0);
    }

    #[test]
    fn test_health_checker() {
        let checker = CacheHealthChecker::new(CacheAlertThresholds::default());
        
        let mut metrics = CacheMetrics::default();
        metrics.hits = 80;
        metrics.misses = 20;
        metrics.avg_get_latency_ms = 50.0;

        let health = checker.check(&metrics);
        assert_eq!(health.status, CacheHealth::Healthy);
        assert!(health.alerts.is_empty());
    }

    #[test]
    fn test_health_checker_low_hit_rate() {
        let checker = CacheHealthChecker::new(CacheAlertThresholds::default());
        
        let mut metrics = CacheMetrics::default();
        metrics.hits = 20;
        metrics.misses = 80;
        metrics.avg_get_latency_ms = 50.0;

        let health = checker.check(&metrics);
        assert_eq!(health.status, CacheHealth::Degraded);
        assert!(!health.alerts.is_empty());
    }
}
