//! Error types for infrastructure integration.

use thiserror::Error;

/// Errors that can occur during infrastructure generation.
#[derive(Error, Debug)]
pub enum InfrastructureError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid configuration: {0}")]
    Config(String),
    #[error("Terraform generation failed: {0}")]
    Terraform(String),
    #[error("Ansible generation failed: {0}")]
    Ansible(String),
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
}