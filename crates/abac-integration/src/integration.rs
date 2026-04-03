//! Integration with RBAC and metadata systems.

use std::sync::Arc;

use crate::error::{AbacError, Result};
use crate::model::{Subject, Resource, Environment};
use crate::policy::PolicyEngine;

/// Integration with RBAC (Role‑Based Access Control).
#[cfg(feature = "rbac")]
pub struct RbacAbacIntegration {
    policy_engine: Arc<PolicyEngine>,
    rbac_manager: Arc<dyn rbac::RbacManager>,
}

#[cfg(feature = "rbac")]
impl RbacAbacIntegration {
    /// Create a new integration.
    pub fn new(policy_engine: Arc<PolicyEngine>, rbac_manager: Arc<dyn rbac::RbacManager>) -> Self {
        Self {
            policy_engine,
            rbac_manager,
        }
    }

    /// Check access using both RBAC and ABAC.
    pub async fn check_access(
        &self,
        subject: &Subject,
        resource: &Resource,
        action: &str,
        environment: &Environment,
    ) -> Result<bool> {
        // First, check RBAC permissions (if subject has a role)
        let subject_id: u64 = subject.id.parse().unwrap_or(0);
        let permission = rbac::Permission::new(&resource.resource_type, action);
        let rbac_allowed = self.rbac_manager.check_permission(subject_id, &permission);

        // If RBAC denies, no need to evaluate ABAC
        if !rbac_allowed {
            return Ok(false);
        }

        // Evaluate ABAC policies
        self.policy_engine.evaluate(subject, resource, action, environment).await
    }
}

/// Integration with metadata management.
#[cfg(feature = "metadata")]
pub struct MetadataAbacIntegration {
    policy_engine: Arc<PolicyEngine>,
    metadata_storage: Arc<dyn metadata_management::MetadataStorage>,
}

#[cfg(feature = "metadata")]
impl MetadataAbacIntegration {
    /// Create a new integration.
    pub fn new(
        policy_engine: Arc<PolicyEngine>,
        metadata_storage: Arc<dyn metadata_management::MetadataStorage>,
    ) -> Self {
        Self {
            policy_engine,
            metadata_storage,
        }
    }

    /// Enrich resource attributes from metadata before evaluation.
    pub async fn enrich_resource(
        &self,
        resource: &mut Resource,
    ) -> Result<()> {
        // Fetch metadata for this resource
        let metadata_list = self.metadata_storage.list_by_entity(&resource.id).await
            .map_err(|e| AbacError::Other(format!("metadata error: {}", e)))?;
        for metadata in metadata_list {
            if let Some(obj) = metadata.content.as_object() {
                for (key, value) in obj {
                    resource.add_attribute(key.clone(), value.clone());
                }
            }
        }
        Ok(())
    }
}

/// Combined access control manager.
pub struct AccessControlManager {
    policy_engine: Arc<PolicyEngine>,
    #[cfg(feature = "rbac")]
    rbac_integration: Option<RbacAbacIntegration>,
    #[cfg(feature = "metadata")]
    metadata_integration: Option<MetadataAbacIntegration>,
}

impl AccessControlManager {
    /// Create a new manager.
    pub fn new(policy_engine: Arc<PolicyEngine>) -> Self {
        Self {
            policy_engine,
            #[cfg(feature = "rbac")]
            rbac_integration: None,
            #[cfg(feature = "metadata")]
            metadata_integration: None,
        }
    }

    /// Set RBAC integration.
    #[cfg(feature = "rbac")]
    pub fn with_rbac(mut self, rbac_integration: RbacAbacIntegration) -> Self {
        self.rbac_integration = Some(rbac_integration);
        self
    }

    /// Set metadata integration.
    #[cfg(feature = "metadata")]
    pub fn with_metadata(mut self, metadata_integration: MetadataAbacIntegration) -> Self {
        self.metadata_integration = Some(metadata_integration);
        self
    }

    /// Check access with all integrated systems.
    pub async fn check_access(
        &self,
        subject: &Subject,
        resource: &Resource,
        action: &str,
        environment: &Environment,
    ) -> Result<bool> {
        // Enrich resource with metadata if available
        let mut enriched_resource = resource.clone();
        #[cfg(feature = "metadata")]
        if let Some(integration) = &self.metadata_integration {
            integration.enrich_resource(&mut enriched_resource).await?;
        }

        // Check RBAC if integrated
        #[cfg(feature = "rbac")]
        if let Some(integration) = &self.rbac_integration {
            return integration.check_access(subject, &enriched_resource, action, environment).await;
        }

        // Fallback to pure ABAC
        self.policy_engine.evaluate(subject, &enriched_resource, action, environment).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_control_manager() {
        let policy_engine = Arc::new(PolicyEngine::new());
        let manager = AccessControlManager::new(policy_engine);
        let subject = Subject::new("agent1", "agent");
        let resource = Resource::new("task1", "task");
        let environment = Environment::new();
        // Should not panic
        let _ = manager.check_access(&subject, &resource, "read", &environment).await;
    }
}