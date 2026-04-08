//! Integration with infrastructure‑as‑code tools (Terraform, Ansible, Pulumi).
//!
//! This crate provides utilities to generate configuration files and
//! deployment scripts for multi‑agent systems.

pub mod terraform;
pub mod ansible;
pub mod pulumi;
pub mod error;
pub mod config;

pub use terraform::TerraformGenerator;
pub use ansible::AnsibleGenerator;
pub use pulumi::PulumiGenerator;
pub use config::DeploymentConfig;
pub use error::InfrastructureError;

/// High‑level manager that orchestrates infrastructure generation.
pub struct InfrastructureManager {
    terraform: TerraformGenerator,
    ansible: AnsibleGenerator,
    pulumi: PulumiGenerator,
}

impl InfrastructureManager {
    /// Create a new manager with default generators.
    pub fn new() -> Self {
        Self {
            terraform: TerraformGenerator::default(),
            ansible: AnsibleGenerator::default(),
            pulumi: PulumiGenerator::default(),
        }
    }

    /// Generate all infrastructure files for a given deployment configuration.
    pub async fn generate_all(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        self.terraform.generate(config, output_dir).await?;
        self.ansible.generate(config, output_dir).await?;
        self.pulumi.generate(config, output_dir).await?;
        Ok(())
    }

    /// Generate only Terraform configuration.
    pub async fn generate_terraform(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        self.terraform.generate(config, output_dir).await
    }

    /// Generate only Ansible configuration.
    pub async fn generate_ansible(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        self.ansible.generate(config, output_dir).await
    }

    /// Generate only Pulumi configuration.
    pub async fn generate_pulumi(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        self.pulumi.generate(config, output_dir).await
    }
}