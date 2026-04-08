//! Pulumi infrastructure‑as‑code generation.
//!
//! This module generates Pulumi programs (TypeScript) for deploying
//! multi‑agent systems to various cloud providers.

use std::fs;
use std::path::Path;
use crate::config::{DeploymentConfig, CloudProvider, AgentSpec};
use crate::error::InfrastructureError;

/// Generates Pulumi programs for a given deployment.
#[derive(Default)]
pub struct PulumiGenerator;

impl PulumiGenerator {
    /// Generate Pulumi files in the specified output directory.
    pub async fn generate(&self, config: &DeploymentConfig, output_dir: &str) -> Result<(), InfrastructureError> {
        let dir = Path::new(output_dir);
        fs::create_dir_all(dir)?;

        // Generate Pulumi.yaml (project file)
        let pulumi_yaml = self.generate_pulumi_yaml(config);
        fs::write(dir.join("Pulumi.yaml"), pulumi_yaml)?;

        // Generate Pulumi.<stack>.yaml (configuration)
        let pulumi_config = self.generate_pulumi_config(config);
        fs::write(dir.join("Pulumi.dev.yaml"), pulumi_config)?;

        // Generate index.ts (main program)
        let index_ts = self.generate_index_ts(config);
        fs::write(dir.join("index.ts"), index_ts)?;

        // Generate package.json
        let package_json = self.generate_package_json();
        fs::write(dir.join("package.json"), package_json)?;

        // Generate tsconfig.json
        let tsconfig = self.generate_tsconfig();
        fs::write(dir.join("tsconfig.json"), tsconfig)?;

        tracing::info!("Pulumi configuration generated in {}", output_dir);
        Ok(())
    }

    fn generate_pulumi_yaml(&self, config: &DeploymentConfig) -> String {
        format!(
            r#"name: offline-first-agents-{}
runtime: nodejs
description: "Deployment of offline‑first multi‑agent system to {}"
"#,
            config.name.to_lowercase().replace(' ', "-"),
            config.provider
        )
    }

    fn generate_pulumi_config(&self, config: &DeploymentConfig) -> String {
        let mut config_yaml = String::from("config:\n");
        
        match config.provider {
            CloudProvider::Aws => {
                config_yaml.push_str("  aws:region: ");
                config_yaml.push_str(&config.region);
                config_yaml.push('\n');
                config_yaml.push_str("  aws:profile: default\n");
            }
            CloudProvider::Azure => {
                config_yaml.push_str("  azure-native:location: ");
                config_yaml.push_str(&config.region);
                config_yaml.push('\n');
                config_yaml.push_str("  azure-native:subscriptionId: \"\" # Fill in your subscription ID\n");
            }
            CloudProvider::Gcp => {
                config_yaml.push_str("  gcp:project: \"\" # Fill in your project ID\n");
                config_yaml.push_str("  gcp:region: ");
                config_yaml.push_str(&config.region);
                config_yaml.push('\n');
            }
            CloudProvider::Kubernetes => {
                config_yaml.push_str("  kubernetes:configPath: \"~/.kube/config\"\n");
            }
            CloudProvider::BareMetal => {
                config_yaml.push_str("  # Bare metal deployment uses local provisioning\n");
            }
        }

        // Add agent count configuration
        for (idx, spec) in config.agents.iter().enumerate() {
            config_yaml.push_str(&format!("  agent{}Count: {}\n", idx, spec.count));
            config_yaml.push_str(&format!("  agent{}InstanceType: \"{}\"\n", idx, spec.instance_type));
        }

        config_yaml
    }

    fn generate_index_ts(&self, config: &DeploymentConfig) -> String {
        let mut index_ts = String::from("import * as pulumi from \"@pulumi/pulumi\";\n");
        
        // Import provider SDKs
        match config.provider {
            CloudProvider::Aws => {
                index_ts.push_str("import * as aws from \"@pulumi/aws\";\n");
                index_ts.push_str("import * as awsx from \"@pulumi/awsx\";\n");
            }
            CloudProvider::Azure => {
                index_ts.push_str("import * as azure from \"@pulumi/azure-native\";\n");
            }
            CloudProvider::Gcp => {
                index_ts.push_str("import * as gcp from \"@pulumi/gcp\";\n");
            }
            CloudProvider::Kubernetes => {
                index_ts.push_str("import * as k8s from \"@pulumi/kubernetes\";\n");
            }
            CloudProvider::BareMetal => {
                index_ts.push_str("// Bare metal deployment uses local provisioning\n");
            }
        }

        index_ts.push_str("\n// Configuration\n");
        index_ts.push_str("const config = new pulumi.Config();\n");

        // Generate resources based on provider
        let resources = self.generate_resources_ts(config);
        index_ts.push_str(&resources);

        // Generate outputs
        index_ts.push_str("\n// Outputs\n");
        index_ts.push_str("export const deploymentName = \"");
        index_ts.push_str(&config.name);
        index_ts.push_str("\";\n");
        index_ts.push_str("export const region = \"");
        index_ts.push_str(&config.region);
        index_ts.push_str("\";\n");

        index_ts
    }

    fn generate_resources_ts(&self, config: &DeploymentConfig) -> String {
        let mut resources = String::new();
        
        match config.provider {
            CloudProvider::Aws => {
                resources.push_str(
                    r#"
// Create a VPC
const vpc = new awsx.ec2.Vpc("agent-vpc", {
    cidrBlock: "10.0.0.0/16",
    numberOfAvailabilityZones: 2,
    subnetSpecs: [
        { type: awsx.ec2.SubnetType.Public, name: "public" },
        { type: awsx.ec2.SubnetType.Private, name: "private" },
    ],
});

// Create security group
const sg = new aws.ec2.SecurityGroup("agent-sg", {
    vpcId: vpc.vpcId,
    description: "Security group for agent instances",
    ingress: [
        {
            protocol: "tcp",
            fromPort: 22,
            toPort: 22,
            cidrBlocks: ["0.0.0.0/0"],
        },
        {
            protocol: "tcp",
            fromPort: 8080,
            toPort: 8080,
            cidrBlocks: ["0.0.0.0/0"],
        },
        {
            protocol: "tcp",
            fromPort: 9090,
            toPort: 9090,
            cidrBlocks: ["0.0.0.0/0"],
        },
    ],
    egress: [
        {
            protocol: "-1",
            fromPort: 0,
            toPort: 0,
            cidrBlocks: ["0.0.0.0/0"],
        },
    ],
});

// Create EC2 instances for agents
"#,
                );

                for (idx, spec) in config.agents.iter().enumerate() {
                    resources.push_str(&format!(
                        r#"
const agentGroup{} = new aws.ec2.Instance("agent-{}-{{}}", {{
    ami: "ami-0c55b159cbfafe1f0", // Ubuntu 20.04 LTS
    instanceType: "{}",
    vpcSecurityGroupIds: [sg.id],
    subnetId: vpc.publicSubnetIds.then(ids => ids[0]),
    tags: {{
        Name: "agent-{}-{{}}",
        Role: "{}",
        Deployment: "{}",
    }},
    userData: `#!/bin/bash
apt-get update
apt-get install -y docker.io
systemctl start docker
systemctl enable docker
docker run -d --name agent -p 8080:8080 -p 9090:9090 \
    -e AGENT_ID={{}} \
    -e AGENT_ROLE={} \
    your-registry/agent:latest`,
    count: config.getNumber("agent{}Count") || {},
}}, {{ count: config.getNumber("agent{}Count") || {} }});
"#,
                        idx, idx, spec.instance_type, idx, spec.role, config.name, idx, spec.role, idx, spec.count, idx, spec.count
                    ));
                }
            }
            CloudProvider::Azure => {
                resources.push_str(
                    r#"
// Create resource group
const resourceGroup = new azure.resources.ResourceGroup("agent-rg", {
    location: config.require("azure-native:location"),
});

// Create virtual network
const vnet = new azure.network.VirtualNetwork("agent-vnet", {
    resourceGroupName: resourceGroup.name,
    location: resourceGroup.location,
    addressSpace: {
        addressPrefixes: ["10.0.0.0/16"],
    },
});

// Create subnet
const subnet = new azure.network.Subnet("agent-subnet", {
    resourceGroupName: resourceGroup.name,
    virtualNetworkName: vnet.name,
    addressPrefix: "10.0.1.0/24",
});

// Create network security group
const nsg = new azure.network.NetworkSecurityGroup("agent-nsg", {
    resourceGroupName: resourceGroup.name,
    location: resourceGroup.location,
    securityRules: [
        {
            name: "ssh",
            priority: 100,
            direction: "Inbound",
            access: "Allow",
            protocol: "Tcp",
            sourcePortRange: "*",
            destinationPortRange: "22",
            sourceAddressPrefix: "*",
            destinationAddressPrefix: "*",
        },
        {
            name: "http",
            priority: 110,
            direction: "Inbound",
            access: "Allow",
            protocol: "Tcp",
            sourcePortRange: "*",
            destinationPortRange: "8080",
            sourceAddressPrefix: "*",
            destinationAddressPrefix: "*",
        },
    ],
});
"#,
                );
            }
            CloudProvider::Gcp => {
                resources.push_str(
                    r#"
// Create network
const network = new gcp.compute.Network("agent-network", {
    autoCreateSubnetworks: false,
});

// Create subnet
const subnet = new gcp.compute.Subnetwork("agent-subnet", {
    ipCidrRange: "10.0.1.0/24",
    network: network.id,
    region: config.require("gcp:region"),
});

// Create firewall rules
const firewall = new gcp.compute.Firewall("agent-firewall", {
    network: network.id,
    allows: [
        {
            protocol: "tcp",
            ports: ["22", "8080", "9090"],
        },
        {
            protocol: "icmp",
        },
    ],
    sourceRanges: ["0.0.0.0/0"],
});
"#,
                );
            }
            CloudProvider::Kubernetes => {
                resources.push_str(
                    r#"
// Create namespace
const ns = new k8s.core.v1.Namespace("agent-ns", {
    metadata: {
        name: "agent-system",
    },
});

// Create deployment for agents
"#,
                );

                for (idx, spec) in config.agents.iter().enumerate() {
                    resources.push_str(&format!(
                        r#"
const agentDeployment{} = new k8s.apps.v1.Deployment("agent-deployment-{}", {{
    metadata: {{
        namespace: ns.metadata.name,
    }},
    spec: {{
        replicas: {},
        selector: {{
            matchLabels: {{
                app: "agent-{}",
            }},
        }},
        template: {{
            metadata: {{
                labels: {{
                    app: "agent-{}",
                }},
            }},
            spec: {{
                containers: [{{
                    name: "agent",
                    image: "your-registry/agent:latest",
                    ports: [{{
                        containerPort: 8080,
                    }}, {{
                        containerPort: 9090,
                    }}],
                    env: [
                        {{ name: "AGENT_ID", value: "agent-{}" }},
                        {{ name: "AGENT_ROLE", value: "{}" }},
                    ],
                }}],
            }},
        }},
    }},
}});
"#,
                        idx, idx, spec.count, idx, idx, idx, spec.role
                    ));
                }
            }
            CloudProvider::BareMetal => {
                resources.push_str(
                    r#"
// Bare metal deployment uses local provisioning
// This would typically use the 'local' or 'null' provider
console.log("Bare metal deployment requires manual provisioning");
"#,
                );
            }
        }

        resources
    }

    fn generate_package_json(&self) -> String {
        r#"{
  "name": "offline-first-agents",
  "version": "1.0.0",
  "description": "Pulumi program for deploying offline-first multi-agent system",
  "main": "index.js",
  "scripts": {
    "build": "tsc",
    "preview": "pulumi preview",
    "up": "pulumi up",
    "destroy": "pulumi destroy"
  },
  "dependencies": {
    "@pulumi/pulumi": "^3.0.0"
  },
  "devDependencies": {
    "@types/node": "^18.0.0",
    "typescript": "^5.0.0"
  }
}"#.to_string()
    }

    fn generate_tsconfig(&self) -> String {
        r#"{
  "compilerOptions": {
    "strict": true,
    "outDir": "bin",
    "target": "es2020",
    "module": "commonjs",
    "moduleResolution": "node",
    "sourceMap": true,
    "experimentalDecorators": true,
    "pretty": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitReturns": true,
    "forceConsistentCasingInFileNames": true,
    "allowSyntheticDefaultImports": true
  },
  "files": [
    "index.ts"
  ]
}"#.to_string()
    }
}