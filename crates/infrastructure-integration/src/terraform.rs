//! Terraform configuration generation.

use std::fs;
use std::path::Path;
use serde_json::json;
use crate::config::{DeploymentConfig, CloudProvider};
use crate::error::InfrastructureError;

/// Generates Terraform configuration files for a given deployment.
#[derive(Default)]
pub struct TerraformGenerator;

impl TerraformGenerator {
    /// Generate Terraform files in the specified output directory.
    pub async fn generate(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        let dir = Path::new(output_dir);
        fs::create_dir_all(dir)?;

        // Generate provider‑specific main.tf
        let main_tf = self.generate_main_tf(config)?;
        fs::write(dir.join("main.tf"), main_tf)?;

        // Generate variables.tf
        let variables_tf = self.generate_variables_tf(config);
        fs::write(dir.join("variables.tf"), variables_tf)?;

        // Generate outputs.tf
        let outputs_tf = self.generate_outputs_tf(config);
        fs::write(dir.join("outputs.tf"), outputs_tf)?;

        // Generate terraform.tfvars (optional)
        let tfvars = self.generate_tfvars(config);
        fs::write(dir.join("terraform.tfvars.json"), tfvars)?;

        tracing::info!("Terraform configuration generated in {}", output_dir);
        Ok(())
    }

    fn generate_main_tf(&self, config: &DeploymentConfig) -> Result<String, InfrastructureError> {
        let provider_block = match config.provider {
            CloudProvider::Aws => {
                format!(
                    r#"provider "aws" {{
  region = "{}"
  default_tags {{
    tags = {{
      {}
    }}
  }}
}}"#,
                    config.region,
                    self.tags_to_tf(&config.tags)
                )
            }
            CloudProvider::Azure => {
                format!(
                    r#"provider "azurerm" {{
  features {{}}
  location = "{}"
  subscription_id = var.subscription_id
}}"#,
                    config.region
                )
            }
            CloudProvider::Gcp => {
                format!(
                    r#"provider "google" {{
  project = var.project_id
  region  = "{}"
}}"#,
                    config.region
                )
            }
            CloudProvider::Kubernetes => {
                r#"provider "kubernetes" {
  config_path = var.kubeconfig
}"#.to_string()
            }
            CloudProvider::BareMetal => {
                // Bare metal uses local provider
                r#"provider "local" {}"#.to_string()
            }
        };

        let resources = self.generate_resources(config);
        Ok(format!("{}\n\n{}", provider_block, resources))
    }

    fn generate_resources(&self, config: &DeploymentConfig) -> String {
        let mut resources = String::new();
        match config.provider {
            CloudProvider::Aws => {
                // VPC
                resources.push_str(&format!(
                    r#"
resource "aws_vpc" "main" {{
  cidr_block = "{}"
  enable_dns_support = true
  enable_dns_hostnames = true
  tags = {{
    Name = "offline-first-vpc"
  }}
}}
"#,
                    config.network.cidr
                ));
                // Subnet
                resources.push_str(
                    r#"
resource "aws_subnet" "main" {
  vpc_id     = aws_vpc.main.id
  cidr_block = "10.0.1.0/24"
  map_public_ip_on_launch = true
  tags = {
    Name = "offline-first-subnet"
  }
}
"#,
                );
                // Security group
                resources.push_str(
                    r#"
resource "aws_security_group" "agent" {
  name        = "agent-sg"
  description = "Allow agent communication"
  vpc_id      = aws_vpc.main.id
}
"#,
                );
                // Security group rules
                for (i, rule) in config.network.security_rules.iter().enumerate() {
                    resources.push_str(&format!(
                        r#"
resource "aws_security_group_rule" "rule{}" {{
  type              = "ingress"
  from_port         = {}
  to_port           = {}
  protocol          = "{}"
  cidr_blocks       = [{}]
  security_group_id = aws_security_group.agent.id
}}
"#,
                        i,
                        rule.from_port,
                        rule.to_port,
                        rule.protocol,
                        rule.cidr_blocks.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
                    ));
                }
                // EC2 instances for agents
                for (idx, spec) in config.agents.iter().enumerate() {
                    for i in 0..spec.count {
                        let instance_name = format!("agent-{}-{}", idx, i);
                        resources.push_str(&format!(
                            r#"
resource "aws_instance" "{}" {{
  ami           = var.ami_id
  instance_type = "{}"
  subnet_id     = aws_subnet.main.id
  vpc_security_group_ids = [aws_security_group.agent.id]
  root_block_device {{
    volume_size = {}
  }}
  user_data = <<-EOF
#!/bin/bash
docker run -d --name agent {} {}
  EOF
  tags = {{
    Name = "{}"
  }}
}}
"#,
                            instance_name,
                            spec.machine_type,
                            spec.disk_size_gb,
                            spec.image,
                            spec.args.join(" "),
                            instance_name
                        ));
                    }
                }
            }
            _ => {
                // Simplified placeholder for other providers
                resources.push_str("# Resource generation for this provider is not yet fully implemented.\n");
            }
        }
        resources
    }

    fn generate_variables_tf(&self, config: &DeploymentConfig) -> String {
        match config.provider {
            CloudProvider::Aws => {
                r#"variable "ami_id" {
  description = "AMI ID for agent instances"
  type        = string
  default     = "ami-0c55b159cbfafe1f0"
}

variable "instance_count" {
  description = "Number of agent instances"
  type        = number
  default     = 3
}
"#.to_string()
            }
            CloudProvider::Azure => {
                r#"variable "subscription_id" {
  description = "Azure subscription ID"
  type        = string
}

variable "resource_group_name" {
  description = "Resource group name"
  type        = string
  default     = "offline-first-rg"
}
"#.to_string()
            }
            _ => "".to_string(),
        }
    }

    fn generate_outputs_tf(&self, config: &DeploymentConfig) -> String {
        match config.provider {
            CloudProvider::Aws => {
                r#"output "vpc_id" {
  value = aws_vpc.main.id
}

output "subnet_id" {
  value = aws_subnet.main.id
}

output "agent_public_ips" {
  value = [for inst in aws_instance.agent : inst.public_ip]
}
"#.to_string()
            }
            _ => "".to_string(),
        }
    }

    fn generate_tfvars(&self, config: &DeploymentConfig) -> String {
        let json = json!({
            "region": config.region,
            "tags": config.tags,
            "agent_count": config.agents.iter().map(|a| a.count).sum::<u32>(),
        });
        serde_json::to_string_pretty(&json).unwrap()
    }

    fn tags_to_tf(&self, tags: &std::collections::HashMap<String, String>) -> String {
        let mut lines = Vec::new();
        for (k, v) in tags {
            lines.push(format!("{} = \"{}\"", k, v));
        }
        lines.join("\n      ")
    }
}