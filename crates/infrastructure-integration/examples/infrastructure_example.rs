//! Example demonstrating infrastructure‑as‑code generation.

use infrastructure_integration::{InfrastructureManager, DeploymentConfig, CloudProvider, AgentSpec};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Infrastructure‑as‑Code Generation Example ===\n");

    // 1. Create a deployment configuration
    let config = DeploymentConfig {
        name: "Offline-First Agent Cluster".to_string(),
        provider: CloudProvider::Aws,
        region: "us-east-1".to_string(),
        agents: vec![
            AgentSpec {
                role: "coordinator".to_string(),
                count: 2,
                instance_type: "t3.medium".to_string(),
                image: "agent-coordinator:latest".to_string(),
            },
            AgentSpec {
                role: "worker".to_string(),
                count: 5,
                instance_type: "t3.small".to_string(),
                image: "agent-worker:latest".to_string(),
            },
        ],
        tags: vec![
            ("Environment".to_string(), "development".to_string()),
            ("Project".to_string(), "offline-first-sdk".to_string()),
        ],
        ..Default::default()
    };

    println!("✓ Deployment configuration created:");
    println!("  - Name: {}", config.name);
    println!("  - Provider: {:?}", config.provider);
    println!("  - Region: {}", config.region);
    println!("  - Total agents: {}", config.agents.iter().map(|a| a.count).sum::<u32>());

    // 2. Create infrastructure manager
    let manager = InfrastructureManager::new();
    println!("\n✓ Infrastructure manager created");

    // 3. Create output directory
    let output_dir = "./generated-infrastructure";
    if Path::new(output_dir).exists() {
        std::fs::remove_dir_all(output_dir)?;
    }
    std::fs::create_dir_all(output_dir)?;
    println!("✓ Output directory created: {}", output_dir);

    // 4. Generate all infrastructure files
    println!("\nGenerating infrastructure files...");
    manager.generate_all(&config, output_dir).await?;
    
    println!("\n✓ Infrastructure files generated:");
    
    // List generated files
    let terraform_dir = format!("{}/terraform", output_dir);
    let ansible_dir = format!("{}/ansible", output_dir);
    let pulumi_dir = format!("{}/pulumi", output_dir);
    
    // Create subdirectories for each tool
    std::fs::create_dir_all(&terraform_dir)?;
    std::fs::create_dir_all(&ansible_dir)?;
    std::fs::create_dir_all(&pulumi_dir)?;
    
    // Generate Terraform files
    println!("  - Terraform:");
    manager.generate_terraform(&config, &terraform_dir).await?;
    let tf_files = std::fs::read_dir(&terraform_dir)?;
    for entry in tf_files {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            println!("    * {}", entry.file_name().to_string_lossy());
        }
    }
    
    // Generate Ansible files
    println!("  - Ansible:");
    manager.generate_ansible(&config, &ansible_dir).await?;
    let ansible_files = std::fs::read_dir(&ansible_dir)?;
    for entry in ansible_files {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            println!("    * {}", entry.file_name().to_string_lossy());
        }
    }
    
    // Generate Pulumi files
    println!("  - Pulumi:");
    manager.generate_pulumi(&config, &pulumi_dir).await?;
    let pulumi_files = std::fs::read_dir(&pulumi_dir)?;
    for entry in pulumi_files {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            println!("    * {}", entry.file_name().to_string_lossy());
        }
    }

    // 5. Show sample content
    println!("\n✓ Sample generated content:");
    
    // Read a sample file
    let sample_file = format!("{}/main.tf", terraform_dir);
    if Path::new(&sample_file).exists() {
        let content = std::fs::read_to_string(&sample_file)?;
        let lines: Vec<&str> = content.lines().take(10).collect();
        println!("\nFirst 10 lines of main.tf:");
        for line in lines {
            println!("  {}", line);
        }
        println!("  ...");
    }

    // Read Pulumi index.ts
    let pulumi_index = format!("{}/index.ts", pulumi_dir);
    if Path::new(&pulumi_index).exists() {
        let content = std::fs::read_to_string(&pulumi_index)?;
        let lines: Vec<&str> = content.lines().take(5).collect();
        println!("\nFirst 5 lines of index.ts:");
        for line in lines {
            println!("  {}", line);
        }
        println!("  ...");
    }

    println!("\n=== Example completed successfully ===");
    println!("\nNext steps:");
    println!("1. Review generated files in '{}'", output_dir);
    println!("2. Customize configuration as needed");
    println!("3. Run 'terraform init && terraform apply' in the terraform directory");
    println!("4. Run 'ansible-playbook -i inventory.yaml deploy_agents.yaml' in the ansible directory");
    println!("5. Run 'npm install && pulumi up' in the pulumi directory");
    
    Ok(())
}