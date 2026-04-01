//! RBAC manager for role and permission management.

use crate::error::{Result, RbacError};
use crate::model::{Permission, Role, UserAssignment, Policy, Rule, Effect};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main RBAC manager.
pub struct RbacManager {
    roles: DashMap<Uuid, Role>,
    assignments: DashMap<crate::common::types::AgentId, UserAssignment>,
    policies: DashMap<Uuid, Policy>,
    role_by_name: DashMap<String, Uuid>,
}

impl RbacManager {
    /// Create a new RBAC manager.
    pub fn new() -> Self {
        Self {
            roles: DashMap::new(),
            assignments: DashMap::new(),
            policies: DashMap::new(),
            role_by_name: DashMap::new(),
        }
    }

    /// Add a role.
    pub fn add_role(&self, role: Role) -> Result<()> {
        if self.role_by_name.contains_key(&role.name) {
            return Err(RbacError::Conflict(format!("Role '{}' already exists", role.name)));
        }
        self.role_by_name.insert(role.name.clone(), role.id);
        self.roles.insert(role.id, role);
        Ok(())
    }

    /// Get a role by ID.
    pub fn get_role(&self, role_id: &Uuid) -> Option<Role> {
        self.roles.get(role_id).map(|r| r.clone())
    }

    /// Get a role by name.
    pub fn get_role_by_name(&self, name: &str) -> Option<Role> {
        self.role_by_name.get(name).and_then(|id| self.get_role(&id))
    }

    /// Delete a role.
    pub fn delete_role(&self, role_id: &Uuid) -> Result<()> {
        if let Some((_, role)) = self.roles.remove(role_id) {
            self.role_by_name.remove(&role.name);
            // Remove from assignments (optional).
            for mut assignment in self.assignments.iter_mut() {
                assignment.role_ids.remove(role_id);
            }
            Ok(())
        } else {
            Err(RbacError::RoleNotFound(role_id.to_string()))
        }
    }

    /// Assign a role to a user.
    pub fn assign_role(&self, user_id: crate::common::types::AgentId, role_id: Uuid) -> Result<()> {
        if !self.roles.contains_key(&role_id) {
            return Err(RbacError::RoleNotFound(role_id.to_string()));
        }
        let mut assignment = self.assignments
            .entry(user_id)
            .or_insert_with(|| UserAssignment::new(user_id));
        assignment.add_role(role_id);
        Ok(())
    }

    /// Revoke a role from a user.
    pub fn revoke_role(&self, user_id: crate::common::types::AgentId, role_id: &Uuid) -> Result<()> {
        if let Some(mut assignment) = self.assignments.get_mut(&user_id) {
            assignment.remove_role(role_id);
        }
        Ok(())
    }

    /// Add a permission directly to a user (extra permission).
    pub fn add_user_permission(&self, user_id: crate::common::types::AgentId, permission: Permission) -> Result<()> {
        let mut assignment = self.assignments
            .entry(user_id)
            .or_insert_with(|| UserAssignment::new(user_id));
        assignment.extra_permissions.insert(permission);
        Ok(())
    }

    /// Deny a permission for a user.
    pub fn deny_user_permission(&self, user_id: crate::common::types::AgentId, permission: Permission) -> Result<()> {
        let mut assignment = self.assignments
            .entry(user_id)
            .or_insert_with(|| UserAssignment::new(user_id));
        assignment.denied_permissions.insert(permission);
        Ok(())
    }

    /// Check if a user has a permission.
    pub fn check_permission(&self, user_id: crate::common::types::AgentId, permission: &Permission) -> bool {
        // Build role map for inheritance.
        let role_map: HashMap<Uuid, Role> = self.roles.iter().map(|r| (*r.key(), r.clone())).collect();
        if let Some(assignment) = self.assignments.get(&user_id) {
            assignment.has_permission(permission, &role_map)
        } else {
            false
        }
    }

    /// Add a policy.
    pub fn add_policy(&self, policy: Policy) -> Result<()> {
        self.policies.insert(policy.id, policy);
        Ok(())
    }

    /// Evaluate a policy for a user and permission.
    pub fn evaluate_policy(&self, user_id: crate::common::types::AgentId, permission: &Permission) -> Effect {
        for policy in self.policies.iter() {
            for rule in &policy.rules {
                // Check subject match (simplified: subject can be "*" or role name).
                if rule.subject == "*" || self.user_has_role(user_id, &rule.subject) {
                    if rule.permission.matches(permission) {
                        return rule.effect.clone();
                    }
                }
            }
        }
        Effect::Deny // default deny
    }

    /// Check if user has a role (by name).
    fn user_has_role(&self, user_id: crate::common::types::AgentId, role_name: &str) -> bool {
        if let Some(assignment) = self.assignments.get(&user_id) {
            for role_id in &assignment.role_ids {
                if let Some(role) = self.roles.get(role_id) {
                    if role.name == role_name {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// List all roles.
    pub fn list_roles(&self) -> Vec<Role> {
        self.roles.iter().map(|r| r.clone()).collect()
    }

    /// List assignments for a user.
    pub fn get_user_assignments(&self, user_id: crate::common::types::AgentId) -> Option<UserAssignment> {
        self.assignments.get(&user_id).map(|a| a.clone())
    }
}

/// Async RBAC manager with distributed capabilities.
pub struct DistributedRbacManager {
    local: Arc<RbacManager>,
    // In a real implementation, would include transport for synchronization.
}

impl DistributedRbacManager {
    /// Create a new distributed RBAC manager.
    pub fn new() -> Self {
        Self {
            local: Arc::new(RbacManager::new()),
        }
    }

    /// Get the local manager.
    pub fn local(&self) -> Arc<RbacManager> {
        self.local.clone()
    }
}