//! Integration with Role‑Based Access Control (RBAC).
//!
//! This module bridges security configuration profiles with RBAC roles and permissions,
//! allowing security policies to be expressed in terms of RBAC entities.
//!
//! Requires the `rbac` feature.

use crate::config::{Policy, PolicyRule, SecurityProfile};
use crate::error::{Result, SecurityConfigError};
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for interacting with an RBAC system.
#[async_trait]
pub trait RbacAdapter: Send + Sync {
    /// Checks whether a subject (agent) has a specific permission on a resource.
    async fn check_permission(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool>;

    /// Returns all roles assigned to a subject.
    async fn get_roles(&self, subject: &str) -> Result<Vec<String>>;

    /// Returns all permissions granted to a role.
    async fn get_role_permissions(&self, role: &str) -> Result<Vec<String>>;
}

/// Adapter that maps RBAC checks to security policy rules.
pub struct RbacPolicyAdapter {
    rbac: Box<dyn RbacAdapter>,
}

impl RbacPolicyAdapter {
    /// Creates a new adapter with the given RBAC backend.
    pub fn new(rbac: Box<dyn RbacAdapter>) -> Self {
        Self { rbac }
    }

    /// Converts an RBAC role‑based policy into a security policy.
    ///
    /// The generated policy will contain rules that check RBAC permissions
    /// for the given role.
    pub async fn role_to_policy(&self, role: &str) -> Result<Policy> {
        let permissions = self.rbac.get_role_permissions(role).await?;
        let mut rules = Vec::new();

        for perm in permissions {
            // Parse permission string (format "action:resource" or similar)
            let parts: Vec<&str> = perm.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let action = parts[0];
            let resource = parts[1];

            rules.push(PolicyRule::Action {
                action: action.to_string(),
                resource: resource.to_string(),
                allow: true,
                conditions: HashMap::new(),
            });
        }

        Ok(Policy {
            id: format!("rbac-role-{}", role),
            name: format!("RBAC role {}", role),
            description: format!("Auto‑generated from RBAC role '{}'", role),
            rules,
            mandatory: false,
            tags: vec!["rbac".to_string()],
        })
    }

    /// Creates a security profile that enforces RBAC for a set of roles.
    pub async fn roles_to_profile(&self, roles: &[String], profile_name: &str) -> Result<SecurityProfile> {
        let mut policies = Vec::new();
        for role in roles {
            let policy = self.role_to_policy(role).await?;
            policies.push(policy);
        }

        Ok(SecurityProfile {
            description: format!("RBAC‑based profile for roles {:?}", roles),
            policies,
            enabled: true,
            priority: 10,
        })
    }

    /// Evaluates a policy rule that references RBAC.
    ///
    /// This method can be used by the policy evaluator to delegate RBAC‑aware rules.
    pub async fn evaluate_rbac_rule(
        &self,
        subject: &str,
        action: &str,
        resource: &str,
    ) -> Result<bool> {
        self.rbac.check_permission(subject, action, resource).await
    }
}

/// Simple in‑memory RBAC adapter for testing.
pub struct InMemoryRbac {
    roles: HashMap<String, Vec<String>>,               // subject → roles
    role_permissions: HashMap<String, Vec<String>>,    // role → permissions
}

impl InMemoryRbac {
    /// Creates a new in‑memory RBAC store.
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            role_permissions: HashMap::new(),
        }
    }

    /// Assigns a role to a subject.
    pub fn assign_role(&mut self, subject: &str, role: &str) {
        self.roles
            .entry(subject.to_string())
            .or_insert_with(Vec::new)
            .push(role.to_string());
    }

    /// Grants a permission to a role.
    pub fn grant_permission(&mut self, role: &str, permission: &str) {
        self.role_permissions
            .entry(role.to_string())
            .or_insert_with(Vec::new)
            .push(permission.to_string());
    }
}

#[async_trait]
impl RbacAdapter for InMemoryRbac {
    async fn check_permission(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool> {
        let roles = self.roles.get(subject).cloned().unwrap_or_default();
        for role in roles {
            let perms = self.role_permissions.get(&role).cloned().unwrap_or_default();
            for perm in perms {
                if perm == permission || perm == format!("{}:{}", permission, resource) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn get_roles(&self, subject: &str) -> Result<Vec<String>> {
        Ok(self.roles.get(subject).cloned().unwrap_or_default())
    }

    async fn get_role_permissions(&self, role: &str) -> Result<Vec<String>> {
        Ok(self.role_permissions.get(role).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_rbac() {
        let mut rbac = InMemoryRbac::new();
        rbac.assign_role("alice", "admin");
        rbac.grant_permission("admin", "read:*");

        assert!(rbac.check_permission("alice", "read", "file").await.unwrap());
        assert!(!rbac.check_permission("bob", "read", "file").await.unwrap());

        let roles = rbac.get_roles("alice").await.unwrap();
        assert_eq!(roles, vec!["admin"]);
    }

    #[tokio::test]
    async fn test_rbac_policy_adapter() {
        let mut rbac = InMemoryRbac::new();
        rbac.assign_role("alice", "viewer");
        rbac.grant_permission("viewer", "read:document");

        let adapter = RbacPolicyAdapter::new(Box::new(rbac));
        let policy = adapter.role_to_policy("viewer").await.unwrap();
        assert_eq!(policy.id, "rbac-role-viewer");
        assert_eq!(policy.rules.len(), 1);
    }
}