//! Secure secrets management for offline‑first multi‑agent systems.
//!
//! This crate provides a unified interface for storing, retrieving, rotating,
//! and distributing secrets across a swarm of agents. It supports multiple
//! backends (in‑memory, encrypted file, Kubernetes Secrets, HashiCorp Vault,
//! AWS Secrets Manager, Azure Key Vault) and integrates with the mesh transport
//! for secure peer‑to‑peer secret distribution.
//!
//! # Features
//!
//! - **Multiple backends**: Choose where secrets are stored.
//! - **Encryption at rest**: All secrets are encrypted before storage.
//! - **Key rotation**: Automatic and manual rotation of encryption keys.
//! - **Access control**: Fine‑grained policies for who can read/write secrets.
//! - **Audit logging**: Track all secret accesses and modifications.
//! - **Distributed caching**: Share secrets across agents securely.
//! - **Integration with mesh transport**: Propagate secrets via secure channels.
//!
//! # Example
//!
//! ```rust,no_run
//! use secrets_management::{SecretsManager, InMemoryBackend, Secret};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = InMemoryBackend::new();
//!     let mut manager = SecretsManager::new(backend);
//!
//!     // Store a secret
//!     let secret = Secret::new(
//!         "database_password",
//!         "supersecret123",
//!         vec!["prod".to_string()],
//!     );
//!     manager.put(secret).await?;
//!
//!     // Retrieve it
//!     let retrieved = manager.get("database_password").await?;
//!     println!("Secret value: {}", retrieved.value());
//!
//!     // Rotate the secret
//!     manager.rotate("database_password", "newpassword456").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod backend;
pub mod crypto;
pub mod error;
pub mod manager;
pub mod model;
pub mod policy;
pub mod rotation;
pub mod transport;

#[cfg(feature = "kubernetes")]
pub mod kubernetes;

#[cfg(feature = "aws")]
pub mod aws;

#[cfg(feature = "azure")]
pub mod azure;

#[cfg(feature = "hashicorp")]
pub mod hashicorp;

pub use backend::{Backend, InMemoryBackend, EncryptedFileBackend};
pub use crypto::{EncryptionKey, KeyManager, KeyRotationStrategy};
pub use error::{SecretsError, Result};
pub use manager::SecretsManager;
pub use model::{Secret, SecretMetadata, SecretVersion, AccessPolicy};
pub use policy::PolicyEngine;
pub use rotation::RotationScheduler;
pub use transport::SecretTransport;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        SecretsManager,
        Backend,
        Secret,
        SecretMetadata,
        SecretsError,
        Result,
        EncryptionKey,
        KeyManager,
    };
}