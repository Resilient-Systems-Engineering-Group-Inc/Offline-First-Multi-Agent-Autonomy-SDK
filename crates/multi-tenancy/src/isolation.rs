//! Tenant isolation strategies.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Tenant isolation level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Shared resources with logical separation
    Shared,
    /// Separate schema per tenant
    Schema,
    /// Separate database per tenant
    Database,
    /// Separate instance per tenant
    Instance,
}

/// Tenant isolation configuration.
pub struct TenantIsolation {
    tenant_id: String,
    isolation_level: IsolationLevel,
    schema_prefix: String,
    database_name: Option<String>,
}

impl TenantIsolation {
    /// Create new tenant isolation.
    pub fn new(tenant_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            isolation_level: IsolationLevel::Schema,
            schema_prefix: format!("tenant_{}", tenant_id.replace('-', "_")),
            database_name: None,
        }
    }

    /// Set isolation level.
    pub fn with_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    /// Get isolation level.
    pub fn get_level(&self) -> &IsolationLevel {
        &self.isolation_level
    }

    /// Get schema name.
    pub fn get_schema_name(&self) -> String {
        self.schema_prefix.clone()
    }

    /// Get database name.
    pub fn get_database_name(&self) -> Option<String> {
        self.database_name.clone()
    }

    /// Set database name.
    pub fn with_database(mut self, db_name: &str) -> Self {
        self.database_name = Some(db_name.to_string());
        self
    }

    /// Build connection string.
    pub fn build_connection_string(&self, base_url: &str) -> Result<String> {
        match self.isolation_level {
            IsolationLevel::Shared => Ok(format!("{}?tenant={}", base_url, self.tenant_id)),
            IsolationLevel::Schema => Ok(format!("{}&options=-c%20search_path%3D{}", base_url, self.get_schema_name())),
            IsolationLevel::Database => {
                let db_name = self.database_name
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Database name not set"))?;
                Ok(format!("{}/{}", base_url, db_name))
            }
            IsolationLevel::Instance => Ok(format!("{}?instance={}", base_url, self.tenant_id)),
        }
    }

    /// Get table name with tenant prefix.
    pub fn get_table_name(&self, table: &str) -> String {
        format!("{}_{}", self.schema_prefix, table)
    }

    /// Add tenant filter to query.
    pub fn add_tenant_filter(&self, query: &str) -> String {
        format!("{} WHERE tenant_id = '{}'", query, self.tenant_id)
    }

    /// Get isolation statistics.
    pub fn get_stats(&self) -> IsolationStats {
        IsolationStats {
            tenant_id: self.tenant_id.clone(),
            isolation_level: format!("{:?}", self.isolation_level),
            schema_name: self.get_schema_name(),
            database_name: self.database_name.clone(),
        }
    }
}

/// Isolation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationStats {
    pub tenant_id: String,
    pub isolation_level: String,
    pub schema_name: String,
    pub database_name: Option<String>,
}

/// Data isolation middleware.
pub struct IsolationMiddleware {
    isolation: TenantIsolation,
}

impl IsolationMiddleware {
    /// Create new isolation middleware.
    pub fn new(tenant_id: &str) -> Self {
        Self {
            isolation: TenantIsolation::new(tenant_id),
        }
    }

    /// Wrap query with tenant isolation.
    pub fn wrap_query(&self, query: &str) -> String {
        self.isolation.add_tenant_filter(query)
    }

    /// Get tenant context.
    pub fn get_tenant_id(&self) -> &str {
        &self.isolation.tenant_id
    }

    /// Get schema context.
    pub fn get_schema(&self) -> &str {
        &self.isolation.schema_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_isolation() {
        let isolation = TenantIsolation::new("tenant-123");
        
        assert_eq!(isolation.get_schema_name(), "tenant_tenant_123");
        assert!(isolation.add_tenant_filter("SELECT * FROM tasks")
            .contains("tenant_id = 'tenant-123'"));
    }

    #[test]
    fn test_table_name() {
        let isolation = TenantIsolation::new("tenant-123");
        
        assert_eq!(isolation.get_table_name("tasks"), "tenant_tenant_123_tasks");
    }

    #[test]
    fn test_isolation_middleware() {
        let middleware = IsolationMiddleware::new("tenant-123");
        
        let query = "SELECT * FROM tasks WHERE id = 1";
        let wrapped = middleware.wrap_query(query);
        
        assert!(wrapped.contains("tenant_id = 'tenant-123'"));
    }
}
