//! Deployment configuration for infrastructure generation.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Cloud provider to deploy onto.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    BareMetal,
    Kubernetes,
}

/// Agent deployment specification.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentSpec {
    /// Number of agent instances.
    pub count: u32,
    /// Machine type (e.g., "t3.micro", "Standard_D2s_v3").
    pub machine_type: String,
    /// Disk size in GB.
    pub disk_size_gb: u32,
    /// Docker image to run.
    pub image: String,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Additional command‑line arguments.
    pub args: Vec<String>,
}

/// Network configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    /// CIDR for VPC/subnet.
    pub cidr: String,
    /// Enable public IPs.
    pub public_ip: bool,
    /// Security group rules.
    pub security_rules: Vec<SecurityRule>,
}

/// Security rule (ingress/egress).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecurityRule {
    pub protocol: String,
    pub from_port: u16,
    pub to_port: u16,
    pub cidr_blocks: Vec<String>,
}

/// Top‑level deployment configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeploymentConfig {
    /// Cloud provider.
    pub provider: CloudProvider,
    /// Region (e.g., "us‑east‑1").
    pub region: String,
    /// Agent specifications.
    pub agents: Vec<AgentSpec>,
    /// Network configuration.
    pub network: NetworkConfig,
    /// Optional Kubernetes namespace (if provider is Kubernetes).
    pub namespace: Option<String>,
    /// Tags for resources.
    pub tags: HashMap<String, String>,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            provider: CloudProvider::Aws,
            region: "us-east-1".to_string(),
            agents: vec![AgentSpec {
                count: 3,
                machine_type: "t3.micro".to_string(),
                disk_size_gb: 20,
                image: "offline‑first‑agent:latest".to_string(),
                env: HashMap::new(),
                args: vec![],
            }],
            network: NetworkConfig {
                cidr: "10.0.0.0/16".to_string(),
                public_ip: true,
                security_rules: vec![
                    SecurityRule {
                        protocol: "tcp".to_string(),
                        from_port: 8080,
                        to_port: 8080,
                        cidr_blocks: vec!["0.0.0.0/0".to_string()],
                    },
                    SecurityRule {
                        protocol: "udp".to_string(),
                        from_port: 6000,
                        to_port: 6100,
                        cidr_blocks: vec!["10.0.0.0/16".to_string()],
                    },
                ],
            },
            namespace: None,
            tags: HashMap::from([
                ("project".to_string(), "offline‑first‑multi‑agent".to_string()),
                ("environment".to_string(), "dev".to_string()),
            ]),
        }
    }
}