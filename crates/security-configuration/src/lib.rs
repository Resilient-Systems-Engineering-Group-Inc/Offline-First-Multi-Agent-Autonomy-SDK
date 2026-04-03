//! Security configuration management for agent systems.
//!
//! This crate provides tools for managing security configurations across
//! a multi‑agent system, including:
//!
//! - **Security profiles** – predefined sets of security policies
//! - **Policy validation** – ensuring configurations are consistent and safe
//! - **Dynamic updates** – hot‑reload of security settings
//! - **Audit logging** – tracking changes to security configurations
//! - **Integration** – with mesh‑transport, RBAC, and other security components
//!
//! # Example
//! ```
//! use security_configuration::{SecurityConfigManager, SecurityProfile, Policy};
//!
//! let manager = SecurityConfigManager::new("config/security.yaml").unwrap();
//! let profile = manager.get_profile("default").unwrap();
//! println!("Profile: {:?}", profile);
//! ```

pub mod config;
pub mod error;
pub mod manager;
pub mod policy;
pub mod profile;
pub mod validator;
pub mod audit;

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "mesh-transport")]
pub mod transport_integration;

#[cfg(feature = "rbac")]
pub mod rbac_integration;

pub use config::*;
pub use error::*;
pub use manager::*;
pub use policy::*;
pub use profile::*;
pub use validator::*;
pub use audit::*;

/// Re‑export of common types for convenience.
pub mod prelude {
    pub use super::{
        SecurityConfig, SecurityProfile, Policy, PolicyRule, SecurityConfigManager,
        ValidationResult, AuditEvent, AuditLogger,
    };
    #[cfg(feature = "crypto")]
    pub use super::crypto::*;
    #[cfg(feature = "mesh-transport")]
    pub use super::transport_integration::*;
    #[cfg(feature = "rbac")]
    pub use super::rbac_integration::*;
}

/// Initializes the security configuration subsystem.
pub fn init() {
    tracing::info!("Security configuration subsystem initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
    }
}