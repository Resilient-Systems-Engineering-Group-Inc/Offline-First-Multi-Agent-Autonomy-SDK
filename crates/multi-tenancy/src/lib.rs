//! Multi-tenancy support for the Multi-Agent SDK.
//!
//! Provides:
//! - Tenant isolation
//! - Resource quotas
//! - Per-tenant configuration
//! - Billing & usage tracking

pub mod tenant;
pub mod isolation;
pub mod quotas;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use tenant::*;
pub use isolation::*;
pub use quotas::*;

/// Multi-tenancy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTenancyConfig {
    pub isolation_level: IsolationLevel,
    pub default_quota: ResourceQuota,
    pub billing_enabled: bool,
    pub max_tenants: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    Shared,
    Schema,
    Database,
    Instance,
}

impl Default for MultiTenancyConfig {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Schema,
            default_quota: ResourceQuota::default(),
            billing_enabled: false,
            max_tenants: 1000,
        }
    }
}

/// Multi-tenancy manager.
pub struct MultiTenancyManager {
    config: MultiTenancyConfig,
    tenants: RwLock<HashMap<String, Tenant>>,
    user_tenants: RwLock<HashMap<String, String>>, // user_id -> tenant_id
}

impl MultiTenancyManager {
    /// Create new multi-tenancy manager.
    pub fn new(config: MultiTenancyConfig) -> Self {
        Self {
            config,
            tenants: RwLock::new(HashMap::new()),
            user_tenants: RwLock::new(HashMap::new()),
        }
    }

    /// Create new tenant.
    pub async fn create_tenant(&self, tenant: &TenantCreateRequest) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;
        
        if tenants.len() >= self.config.max_tenants {
            return Err(anyhow::anyhow!("Maximum number of tenants reached"));
        }

        let tenant_id = uuid::Uuid::new_v4().to_string();
        let new_tenant = Tenant {
            id: tenant_id.clone(),
            name: tenant.name.clone(),
            status: TenantStatus::Active,
            quota: self.config.default_quota.clone(),
            usage: TenantUsage::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        tenants.insert(tenant_id.clone(), new_tenant.clone());
        
        info!("Tenant created: {}", tenant_id);
        Ok(new_tenant)
    }

    /// Get tenant by ID.
    pub async fn get_tenant(&self, tenant_id: &str) -> Result<Tenant> {
        let tenants = self.tenants.read().await;
        
        tenants.get(tenant_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Tenant not found: {}", tenant_id))
    }

    /// Update tenant.
    pub async fn update_tenant(&self, tenant_id: &str, updates: TenantUpdateRequest) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;
        
        let tenant = tenants.get_mut(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Tenant not found: {}", tenant_id))?;

        if let Some(name) = updates.name {
            tenant.name = name;
        }
        
        if let Some(quota) = updates.quota {
            tenant.quota = quota;
        }

        tenant.updated_at = chrono::Utc::now();
        
        info!("Tenant updated: {}", tenant_id);
        Ok(tenant.clone())
    }

    /// Delete tenant.
    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<()> {
        let mut tenants = self.tenants.write().await;
        
        if tenants.remove(tenant_id).is_some() {
            info!("Tenant deleted: {}", tenant_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Tenant not found: {}", tenant_id))
        }
    }

    /// List all tenants.
    pub async fn list_tenants(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read().await;
        tenants.values().cloned().collect()
    }

    /// Assign user to tenant.
    pub async fn assign_user_to_tenant(&self, user_id: &str, tenant_id: &str) -> Result<()> {
        // Verify tenant exists
        self.get_tenant(tenant_id).await?;

        let mut user_tenants = self.user_tenants.write().await;
        user_tenants.insert(user_id.to_string(), tenant_id.to_string());
        
        info!("User {} assigned to tenant {}", user_id, tenant_id);
        Ok(())
    }

    /// Get user's tenant.
    pub async fn get_user_tenant(&self, user_id: &str) -> Option<String> {
        let user_tenants = self.user_tenants.read().await;
        user_tenants.get(user_id).cloned()
    }

    /// Check if user has access to tenant.
    pub async fn has_tenant_access(&self, user_id: &str, tenant_id: &str) -> bool {
        self.get_user_tenant(user_id).await
            .map(|t| t == tenant_id)
            .unwrap_or(false)
    }

    /// Update tenant usage.
    pub async fn update_usage(&self, tenant_id: &str, resource: &str, delta: i64) -> Result<()> {
        let mut tenants = self.tenants.write().await;
        
        let tenant = tenants.get_mut(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Tenant not found: {}", tenant_id))?;

        tenant.usage.update(resource, delta)?;
        tenant.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Check quota limits.
    pub async fn check_quota(&self, tenant_id: &str, resource: &str, amount: i64) -> Result<bool> {
        let tenants = self.tenants.read().await;
        
        let tenant = tenants.get(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Tenant not found: {}", tenant_id))?;

        Ok(tenant.usage.can_allocate(resource, amount, &tenant.quota))
    }

    /// Get tenant statistics.
    pub async fn get_tenant_stats(&self, tenant_id: &str) -> Result<TenantStats> {
        let tenants = self.tenants.read().await;
        
        let tenant = tenants.get(tenant_id)
            .ok_or_else(|| anyhow::anyhow!("Tenant not found: {}", tenant_id))?;

        Ok(TenantStats {
            tenant_id: tenant.id.clone(),
            name: tenant.name.clone(),
            status: tenant.status.clone(),
            quota: tenant.quota.clone(),
            usage: tenant.usage.clone(),
            created_at: tenant.created_at,
            updated_at: tenant.updated_at,
            utilization_percent: tenant.usage.calculate_utilization(&tenant.quota),
        })
    }

    /// Get all tenant statistics.
    pub async fn get_all_stats(&self) -> Vec<TenantStats> {
        let tenants = self.tenants.read().await;
        
        tenants.values()
            .map(|t| TenantStats {
                tenant_id: t.id.clone(),
                name: t.name.clone(),
                status: t.status.clone(),
                quota: t.quota.clone(),
                usage: t.usage.clone(),
                created_at: t.created_at,
                updated_at: t.updated_at,
                utilization_percent: t.usage.calculate_utilization(&t.quota),
            })
            .collect()
    }
}

/// Tenant create request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantCreateRequest {
    pub name: String,
    pub email: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Tenant update request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TenantUpdateRequest {
    pub name: Option<String>,
    pub quota: Option<ResourceQuota>,
    pub metadata: Option<serde_json::Value>,
}

/// Tenant statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantStats {
    pub tenant_id: String,
    pub name: String,
    pub status: TenantStatus,
    pub quota: ResourceQuota,
    pub usage: TenantUsage,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub utilization_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_lifecycle() {
        let config = MultiTenancyConfig::default();
        let manager = MultiTenancyManager::new(config);

        // Create tenant
        let create_req = TenantCreateRequest {
            name: "Test Tenant".to_string(),
            email: Some("test@example.com".to_string()),
            metadata: None,
        };

        let tenant = manager.create_tenant(&create_req).await.unwrap();
        assert!(!tenant.id.is_empty());

        // Get tenant
        let fetched = manager.get_tenant(&tenant.id).await.unwrap();
        assert_eq!(fetched.name, "Test Tenant");

        // Update tenant
        let update_req = TenantUpdateRequest {
            name: Some("Updated Tenant".to_string()),
            ..Default::default()
        };

        let updated = manager.update_tenant(&tenant.id, update_req).await.unwrap();
        assert_eq!(updated.name, "Updated Tenant");

        // Delete tenant
        manager.delete_tenant(&tenant.id).await.unwrap();
        
        // Verify deleted
        assert!(manager.get_tenant(&tenant.id).await.is_err());
    }

    #[tokio::test]
    async fn test_user_tenant_assignment() {
        let config = MultiTenancyConfig::default();
        let manager = MultiTenancyManager::new(config);

        // Create tenant
        let create_req = TenantCreateRequest {
            name: "Test Tenant".to_string(),
            email: None,
            metadata: None,
        };

        let tenant = manager.create_tenant(&create_req).await.unwrap();

        // Assign user
        manager.assign_user_to_tenant("user-1", &tenant.id).await.unwrap();

        // Verify assignment
        let user_tenant = manager.get_user_tenant("user-1").await.unwrap();
        assert_eq!(user_tenant, tenant.id);

        // Check access
        let has_access = manager.has_tenant_access("user-1", &tenant.id).await;
        assert!(has_access);
    }

    #[tokio::test]
    async fn test_quota_enforcement() {
        let config = MultiTenancyConfig::default();
        let manager = MultiTenancyManager::new(config);

        // Create tenant with limited quota
        let mut quota = ResourceQuota::default();
        quota.max_tasks = 10;
        
        let create_req = TenantCreateRequest {
            name: "Test Tenant".to_string(),
            email: None,
            metadata: None,
        };

        let tenant = manager.create_tenant(&create_req).await.unwrap();
        
        // Update quota
        manager.update_tenant(&tenant.id, TenantUpdateRequest {
            quota: Some(quota),
            ..Default::default()
        }).await.unwrap();

        // Check quota
        let can_allocate = manager.check_quota(&tenant.id, "tasks", 5).await.unwrap();
        assert!(can_allocate);

        let can_allocate = manager.check_quota(&tenant.id, "tasks", 15).await.unwrap();
        assert!(!can_allocate);
    }
}
