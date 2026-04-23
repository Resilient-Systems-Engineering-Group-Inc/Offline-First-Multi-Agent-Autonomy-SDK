//! Tenant management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::isolation::TenantIsolation;
use crate::quotas::{ResourceQuota, TenantUsage};

/// Tenant status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TenantStatus {
    Active,
    Suspended,
    Deleted,
    Provisioning,
}

/// Tenant entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub status: TenantStatus,
    pub quota: ResourceQuota,
    pub usage: TenantUsage,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    /// Check if tenant is active.
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }

    /// Check if tenant is suspended.
    pub fn is_suspended(&self) -> bool {
        self.status == TenantStatus::Suspended
    }

    /// Suspend tenant.
    pub fn suspend(&mut self) {
        self.status = TenantStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Activate tenant.
    pub fn activate(&mut self) {
        self.status = TenantStatus::Active;
        self.updated_at = Utc::now();
    }

    /// Delete tenant.
    pub fn delete(&mut self) {
        self.status = TenantStatus::Deleted;
        self.updated_at = Utc::now();
    }

    /// Get isolation configuration.
    pub fn get_isolation(&self) -> TenantIsolation {
        TenantIsolation::new(&self.id)
    }

    /// Check if resource allocation is allowed.
    pub fn can_allocate(&self, resource: &str, amount: i64) -> bool {
        self.usage.can_allocate(resource, amount, &self.quota)
    }

    /// Update usage.
    pub fn update_usage(&mut self, resource: &str, delta: i64) -> anyhow::Result<()> {
        self.usage.update(resource, delta)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get utilization percentage.
    pub fn get_utilization(&self) -> f64 {
        self.usage.calculate_utilization(&self.quota)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_status() {
        let mut tenant = Tenant {
            id: "tenant-1".to_string(),
            name: "Test".to_string(),
            status: TenantStatus::Active,
            quota: ResourceQuota::default(),
            usage: TenantUsage::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(tenant.is_active());
        
        tenant.suspend();
        assert!(tenant.is_suspended());
        
        tenant.activate();
        assert!(tenant.is_active());
    }
}
