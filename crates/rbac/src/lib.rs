//! Role‑Based Access Control (RBAC) for offline‑first multi‑agent systems.
//!
//! This crate provides a flexible RBAC system with roles, permissions, policies,
//! and user assignments. It supports role inheritance, custom permissions, and
//! distributed synchronization.
//!
//! # Quick Start
//!
//! ```no_run
//! use rbac::{RbacManager, Role, Permission};
//!
//! let manager = RbacManager::new();
//! let mut admin_role = Role::new("admin", "Administrator");
//! admin_role.add_permission(Permission::new("task", "create"));
//! admin_role.add_permission(Permission::new("task", "delete"));
//! manager.add_role(admin_role).unwrap();
//!
//! manager.assign_role(42, manager.get_role_by_name("admin").unwrap().id).unwrap();
//! let allowed = manager.check_permission(42, &Permission::new("task", "create"));
//! assert!(allowed);
//! ```

pub mod error;
pub mod manager;
pub mod model;

pub use error::*;
pub use manager::*;
pub use model::*;