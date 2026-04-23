//! Resource quotas and usage tracking.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource quota.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceQuota {
    pub max_tasks: i64,
    pub max_agents: i64,
    pub max_workflows: i64,
    pub max_storage_gb: i64,
    pub max_bandwidth_gb: i64,
    pub max_api_calls_per_day: i64,
    pub max_concurrent_connections: i64,
}

impl ResourceQuota {
    /// Check if resource has capacity.
    pub fn has_capacity(&self, resource: &str, amount: i64) -> bool {
        let current_limit = self.get_limit(resource);
        
        if current_limit < 0 {
            return true; // Unlimited
        }

        amount <= current_limit
    }

    /// Get limit for resource.
    pub fn get_limit(&self, resource: &str) -> i64 {
        match resource {
            "tasks" => self.max_tasks,
            "agents" => self.max_agents,
            "workflows" => self.max_workflows,
            "storage" => self.max_storage_gb,
            "bandwidth" => self.max_bandwidth_gb,
            "api_calls" => self.max_api_calls_per_day,
            "connections" => self.max_concurrent_connections,
            _ => i64::MAX,
        }
    }
}

/// Tenant usage.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TenantUsage {
    pub tasks: i64,
    pub agents: i64,
    pub workflows: i64,
    pub storage_gb: i64,
    pub bandwidth_gb: i64,
    pub api_calls_today: i64,
    pub active_connections: i64,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

impl TenantUsage {
    /// Update usage for a resource.
    pub fn update(&mut self, resource: &str, delta: i64) -> Result<()> {
        match resource {
            "tasks" => self.tasks = self.tasks.saturating_add(delta),
            "agents" => self.agents = self.agents.saturating_add(delta),
            "workflows" => self.workflows = self.workflows.saturating_add(delta),
            "storage" => self.storage_gb = self.storage_gb.saturating_add(delta),
            "bandwidth" => self.bandwidth_gb = self.bandwidth_gb.saturating_add(delta),
            "api_calls" => self.api_calls_today = self.api_calls_today.saturating_add(delta),
            "connections" => self.active_connections = self.active_connections.saturating_add(delta),
            _ => return Err(anyhow::anyhow!("Unknown resource: {}", resource)),
        }

        self.last_updated = Some(chrono::Utc::now());
        Ok(())
    }

    /// Check if allocation is allowed.
    pub fn can_allocate(&self, resource: &str, amount: i64, quota: &ResourceQuota) -> bool {
        let current = self.get_usage(resource);
        let limit = quota.get_limit(resource);

        if limit < 0 {
            return true; // Unlimited
        }

        let total = current.saturating_add(amount);
        total <= limit
    }

    /// Get current usage for resource.
    pub fn get_usage(&self, resource: &str) -> i64 {
        match resource {
            "tasks" => self.tasks,
            "agents" => self.agents,
            "workflows" => self.workflows,
            "storage" => self.storage_gb,
            "bandwidth" => self.bandwidth_gb,
            "api_calls" => self.api_calls_today,
            "connections" => self.active_connections,
            _ => 0,
        }
    }

    /// Calculate utilization percentage.
    pub fn calculate_utilization(&self, quota: &ResourceQuota) -> f64 {
        let resources = [
            ("tasks", &quota.max_tasks, &self.tasks),
            ("agents", &quota.max_agents, &self.agents),
            ("workflows", &quota.max_workflows, &self.workflows),
        ];

        let mut total_utilization = 0.0;
        let mut count = 0;

        for (_, limit, usage) in resources.iter() {
            if *limit > 0 {
                let utilization = (*usage as f64 / *limit as f64) * 100.0;
                total_utilization += utilization;
                count += 1;
            }
        }

        if count > 0 {
            total_utilization / count as f64
        } else {
            0.0
        }
    }

    /// Get usage as map.
    pub fn to_map(&self) -> HashMap<String, i64> {
        let mut map = HashMap::new();
        map.insert("tasks".to_string(), self.tasks);
        map.insert("agents".to_string(), self.agents);
        map.insert("workflows".to_string(), self.workflows);
        map.insert("storage_gb".to_string(), self.storage_gb);
        map.insert("bandwidth_gb".to_string(), self.bandwidth_gb);
        map.insert("api_calls_today".to_string(), self.api_calls_today);
        map.insert("active_connections".to_string(), self.active_connections);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_quota() {
        let quota = ResourceQuota {
            max_tasks: 100,
            max_agents: 10,
            max_workflows: 50,
            max_storage_gb: 1000,
            max_bandwidth_gb: 500,
            max_api_calls_per_day: 100000,
            max_concurrent_connections: 100,
        };

        assert!(quota.has_capacity("tasks", 50));
        assert!(!quota.has_capacity("tasks", 150));
        assert!(quota.has_capacity("tasks", 100));
    }

    #[test]
    fn test_tenant_usage() {
        let mut usage = TenantUsage::default();
        
        usage.update("tasks", 10).unwrap();
        assert_eq!(usage.tasks, 10);

        usage.update("tasks", 5).unwrap();
        assert_eq!(usage.tasks, 15);

        usage.update("tasks", -5).unwrap();
        assert_eq!(usage.tasks, 10);
    }

    #[test]
    fn test_allocation_check() {
        let quota = ResourceQuota {
            max_tasks: 100,
            ..Default::default()
        };

        let mut usage = TenantUsage::default();
        usage.update("tasks", 50).unwrap();

        assert!(usage.can_allocate("tasks", 40, &quota));
        assert!(!usage.can_allocate("tasks", 60, &quota));
    }

    #[test]
    fn test_utilization() {
        let quota = ResourceQuota {
            max_tasks: 100,
            max_agents: 10,
            max_workflows: 50,
            ..Default::default()
        };

        let mut usage = TenantUsage::default();
        usage.update("tasks", 50).unwrap();
        usage.update("agents", 5).unwrap();
        usage.update("workflows", 25).unwrap();

        let utilization = usage.calculate_utilization(&quota);
        assert!((utilization - 50.0).abs() < 0.01); // ~50%
    }
}
