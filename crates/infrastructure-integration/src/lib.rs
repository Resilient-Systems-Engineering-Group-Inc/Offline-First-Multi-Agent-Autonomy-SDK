//! Integration with infrastructure‑as‑code tools (Terraform, Ansible).
//!
//! This crate provides utilities to generate configuration files and
//! deployment scripts for multi‑agent systems.

pub mod terraform;
pub mod ansible;
pub mod error;
pub mod config;

pub use terraform::TerraformGenerator;
pub use ansible::AnsibleGenerator;
pub use config::DeploymentConfig;
pub use error::InfrastructureError;

/// High‑level manager that orchestrates infrastructure generation.
pub struct InfrastructureManager {
    terraform: TerraformGenerator,
    ansible: AnsibleGenerator,
}

impl InfrastructureManager {
    /// Create a new manager with default generators.
    pub fn new() -> Self {
        Self {
            terraform: TerraformGenerator::default(),
            ansible: AnsibleGenerator::default(),
        }
    }

    /// Generate all infrastructure files for a given deployment configuration.
    pub async fn generate_all(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        self.terraform.generate(config, output_dir).await?;
        self.ansible.generate(config, output_dir).await?;
        Ok(())
    }
}