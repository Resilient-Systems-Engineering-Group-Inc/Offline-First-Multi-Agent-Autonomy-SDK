//! Role-Based Access Control (RBAC).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Resource types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Task,
    Workflow,
    Agent,
    System,
    AuditLog,
}

/// Action types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    Execute,
    Pause,
    Resume,
    Cancel,
    Admin,
}

/// Permission definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub resource: ResourceType,
    pub actions: Vec<Action>,
}

/// Role definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
    pub description: Option<String>,
}

impl Role {
    /// Create new role.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            permissions: vec![],
            description: None,
        }
    }

    /// Add permission to role.
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.push(permission);
        self
    }

    /// Check if role has permission.
    pub fn has_permission(&self, resource: &ResourceType, action: &Action) -> bool {
        self.permissions.iter().any(|p| {
            p.resource == *resource && p.actions.contains(action)
        })
    }
}

/// Predefined roles.
pub mod predefined_roles {
    use super::*;

    /// Admin role with full access.
    pub fn admin() -> Role {
        Role::new("admin")
            .with_permission(Permission {
                resource: ResourceType::Task,
                actions: vec![
                    Action::Create, Action::Read, Action::Update,
                    Action::Delete, Action::Execute, Action::Admin
                ],
            })
            .with_permission(Permission {
                resource: ResourceType::Workflow,
                actions: vec![
                    Action::Create, Action::Read, Action::Update,
                    Action::Delete, Action::Execute, Action::Pause,
                    Action::Resume, Action::Cancel, Action::Admin
                ],
            })
            .with_permission(Permission {
                resource: ResourceType::Agent,
                actions: vec![
                    Action::Create, Action::Read, Action::Update,
                    Action::Delete, Action::Admin
                ],
            })
            .with_permission(Permission {
                resource: ResourceType::System,
                actions: vec![Action::Admin],
            })
            .with_permission(Permission {
                resource: ResourceType::AuditLog,
                actions: vec![Action::Read, Action::Admin],
            })
    }

    /// Operator role for managing tasks and workflows.
    pub fn operator() -> Role {
        Role::new("operator")
            .with_permission(Permission {
                resource: ResourceType::Task,
                actions: vec![
                    Action::Create, Action::Read, Action::Update,
                    Action::Execute, Action::Cancel
                ],
            })
            .with_permission(Permission {
                resource: ResourceType::Workflow,
                actions: vec![
                    Action::Create, Action::Read, Action::Update,
                    Action::Execute, Action::Pause, Action::Resume,
                    Action::Cancel
                ],
            })
            .with_permission(Permission {
                resource: ResourceType::Agent,
                actions: vec![Action::Read],
            })
    }

    /// Viewer role with read-only access.
    pub fn viewer() -> Role {
        Role::new("viewer")
            .with_permission(Permission {
                resource: ResourceType::Task,
                actions: vec![Action::Read],
            })
            .with_permission(Permission {
                resource: ResourceType::Workflow,
                actions: vec![Action::Read],
            })
            .with_permission(Permission {
                resource: ResourceType::Agent,
                actions: vec![Action::Read],
            })
            .with_permission(Permission {
                resource: ResourceType::AuditLog,
                actions: vec![Action::Read],
            })
    }

    /// Agent role for executing tasks.
    pub fn agent() -> Role {
        Role::new("agent")
            .with_permission(Permission {
                resource: ResourceType::Task,
                actions: vec![Action::Read, Action::Update, Action::Execute],
            })
            .with_permission(Permission {
                resource: ResourceType::Workflow,
                actions: vec![Action::Read],
            })
    }
}

/// RBAC manager.
pub struct RbacManager {
    roles: Arc<RwLock<HashMap<String, Role>>>,
}

impl RbacManager {
    /// Create new RBAC manager.
    pub fn new() -> Self {
        let mut roles = HashMap::new();

        // Add predefined roles
        roles.insert("admin".to_string(), predefined_roles::admin());
        roles.insert("operator".to_string(), predefined_roles::operator());
        roles.insert("viewer".to_string(), predefined_roles::viewer());
        roles.insert("agent".to_string(), predefined_roles::agent());

        Self {
            roles: Arc::new(RwLock::new(roles)),
        }
    }

    /// Check if user has permission.
    pub async fn check_permission(
        &self,
        user_roles: &[String],
        resource: &ResourceType,
        action: &Action,
    ) -> bool {
        let roles = self.roles.read().await;

        for role_name in user_roles {
            if let Some(role) = roles.get(role_name) {
                if role.has_permission(resource, action) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if user has any of the specified roles.
    pub async fn has_any_role(&self, user_roles: &[String], required_roles: &[String]) -> bool {
        for user_role in user_roles {
            if required_roles.contains(user_role) {
                return true;
            }
        }
        false
    }

    /// Check if user has all specified roles.
    pub async fn has_all_roles(&self, user_roles: &[String], required_roles: &[String]) -> bool {
        required_roles.iter().all(|r| user_roles.contains(r))
    }

    /// Add custom role.
    pub async fn add_role(&self, role: Role) {
        let mut roles = self.roles.write().await;
        roles.insert(role.name.clone(), role);
    }

    /// Get role by name.
    pub async fn get_role(&self, name: &str) -> Option<Role> {
        let roles = self.roles.read().await;
        roles.get(name).cloned()
    }

    /// List all roles.
    pub async fn list_roles(&self) -> Vec<String> {
        let roles = self.roles.read().await;
        roles.keys().cloned().collect()
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rbac_permissions() {
        let rbac = RbacManager::new();

        // Admin should have all permissions
        assert!(rbac.check_permission(
            &["admin".to_string()],
            &ResourceType::Task,
            &Action::Delete
        ).await);

        // Viewer should not have delete permission
        assert!(!rbac.check_permission(
            &["viewer".to_string()],
            &ResourceType::Task,
            &Action::Delete
        ).await);

        // Viewer should have read permission
        assert!(rbac.check_permission(
            &["viewer".to_string()],
            &ResourceType::Task,
            &Action::Read
        ).await);

        // Agent should be able to execute tasks
        assert!(rbac.check_permission(
            &["agent".to_string()],
            &ResourceType::Task,
            &Action::Execute
        ).await);

        // Agent should not be able to delete tasks
        assert!(!rbac.check_permission(
            &["agent".to_string()],
            &ResourceType::Task,
            &Action::Delete
        ).await);
    }

    #[tokio::test]
    async fn test_role_composition() {
        let rbac = RbacManager::new();

        // User with multiple roles
        let user_roles = vec!["viewer".to_string(), "operator".to_string()];

        // Should have operator permissions
        assert!(rbac.check_permission(
            &user_roles,
            &ResourceType::Task,
            &Action::Create
        ).await);

        // Should not have admin permissions
        assert!(!rbac.check_permission(
            &user_roles,
            &ResourceType::System,
            &Action::Admin
        ).await);
    }
}
