//! RBAC data models.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A permission represents an action on a resource.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// Resource identifier (e.g., "task", "agent", "config").
    pub resource: String,
    /// Action (e.g., "read", "write", "delete", "execute").
    pub action: String,
    /// Optional constraint (e.g., "owner", "team").
    pub constraint: Option<String>,
}

impl Permission {
    /// Create a new permission.
    pub fn new(resource: &str, action: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            constraint: None,
        }
    }

    /// Create a permission with constraint.
    pub fn with_constraint(resource: &str, action: &str, constraint: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            constraint: Some(constraint.to_string()),
        }
    }

    /// Check if this permission matches another (ignoring constraint).
    pub fn matches(&self, other: &Permission) -> bool {
        self.resource == other.resource && self.action == other.action
    }
}

/// A role is a collection of permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique role ID.
    pub id: Uuid,
    /// Role name (e.g., "admin", "user", "viewer").
    pub name: String,
    /// Description.
    pub description: String,
    /// Set of permissions granted by this role.
    pub permissions: HashSet<Permission>,
    /// Parent roles (inheritance).
    pub parents: HashSet<Uuid>,
}

impl Role {
    /// Create a new role.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            permissions: HashSet::new(),
            parents: HashSet::new(),
        }
    }

    /// Add a permission.
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }

    /// Remove a permission.
    pub fn remove_permission(&mut self, permission: &Permission) {
        self.permissions.remove(permission);
    }

    /// Add a parent role.
    pub fn add_parent(&mut self, parent_id: Uuid) {
        self.parents.insert(parent_id);
    }

    /// Check if this role has a specific permission (including inheritance).
    pub fn has_permission(&self, permission: &Permission, role_map: &HashMap<Uuid, Role>) -> bool {
        if self.permissions.contains(permission) {
            return true;
        }
        for parent_id in &self.parents {
            if let Some(parent) = role_map.get(parent_id) {
                if parent.has_permission(permission, role_map) {
                    return true;
                }
            }
        }
        false
    }
}

/// A user (agent) assignment to roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAssignment {
    /// User/agent ID.
    pub user_id: crate::common::types::AgentId,
    /// Assigned role IDs.
    pub role_ids: HashSet<Uuid>,
    /// Custom permissions (additional to roles).
    pub extra_permissions: HashSet<Permission>,
    /// Denied permissions (overrides).
    pub denied_permissions: HashSet<Permission>,
}

impl UserAssignment {
    /// Create a new assignment.
    pub fn new(user_id: crate::common::types::AgentId) -> Self {
        Self {
            user_id,
            role_ids: HashSet::new(),
            extra_permissions: HashSet::new(),
            denied_permissions: HashSet::new(),
        }
    }

    /// Add a role.
    pub fn add_role(&mut self, role_id: Uuid) {
        self.role_ids.insert(role_id);
    }

    /// Remove a role.
    pub fn remove_role(&mut self, role_id: &Uuid) {
        self.role_ids.remove(role_id);
    }

    /// Check if the user has a permission given a role map.
    pub fn has_permission(&self, permission: &Permission, role_map: &HashMap<Uuid, Role>) -> bool {
        // Check denied first (deny overrides).
        if self.denied_permissions.contains(permission) {
            return false;
        }
        // Check extra permissions.
        if self.extra_permissions.contains(permission) {
            return true;
        }
        // Check roles.
        for role_id in &self.role_ids {
            if let Some(role) = role_map.get(role_id) {
                if role.has_permission(permission, role_map) {
                    return true;
                }
            }
        }
        false
    }
}

/// A policy defines access control rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy ID.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Description.
    pub description: String,
    /// List of rules.
    pub rules: Vec<Rule>,
}

/// A rule within a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Subject (user ID, role, or "*" for any).
    pub subject: String,
    /// Permission.
    pub permission: Permission,
    /// Effect (Allow or Deny).
    pub effect: Effect,
    /// Conditions (optional JSON).
    pub conditions: Option<serde_json::Value>,
}

/// Effect of a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Allow,
    Deny,
}