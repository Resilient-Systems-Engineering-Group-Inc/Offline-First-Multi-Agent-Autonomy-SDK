//! Workflow commands.

use crate::commands::CommandResult;
use crate::Commands;
use anyhow::Result;
use console::style;

pub async fn execute(cmd: Commands, api_url: &str) -> CommandResult {
    match cmd {
        Commands::Workflow(WorkflowCommands::List) => list_workflows(api_url).await,
        Commands::Workflow(WorkflowCommands::Create { name, file }) => {
            create_workflow(api_url, &name, file.as_deref()).await
        }
        Commands::Workflow(WorkflowCommands::Start { id }) => start_workflow(api_url, &id).await,
        Commands::Workflow(WorkflowCommands::Status { id }) => get_workflow_status(api_url, &id).await,
        _ => Err("Unknown command".into()),
    }
}

async fn list_workflows(api_url: &str) -> CommandResult {
    let response = reqwest::get(format!("{}/api/workflows", api_url)).await?;
    let workflows: serde_json::Value = response.json().await?;

    println!("\n{} Found {} workflow(s):\n", "✅", workflows.as_array().map(|a| a.len()).unwrap_or(0));

    if let Some(workflows) = workflows.as_array() {
        for workflow in workflows {
            println!(
                "  {} {} v{} - {}",
                style(&workflow["id"]).cyan(),
                style(&workflow["name"]).white(),
                workflow["version"],
                workflow["description"].as_str().unwrap_or("No description")
            );
        }
    }

    Ok(())
}

async fn create_workflow(api_url: &str, name: &str, file: Option<&str>) -> CommandResult {
    let payload = if let Some(file) = file {
        let content = tokio::fs::read_to_string(file).await?;
        serde_json::json!({
            "name": name,
            "yamlDefinition": content
        })
    } else {
        serde_json::json!({
            "name": name,
            "version": "1.0.0"
        })
    };

    let response = reqwest::Client::new()
        .post(format!("{}/api/workflows", api_url))
        .json(&payload)
        .send()
        .await?;

    let workflow: serde_json::Value = response.json().await?;

    println!("\n{} Workflow created:\n", "✅");
    println!("  ID:      {}", workflow["id"]);
    println!("  Name:    {}", workflow["name"]);
    println!("  Version: {}", workflow["version"]);

    Ok(())
}

async fn start_workflow(api_url: &str, id: &str) -> CommandResult {
    let response = reqwest::Client::new()
        .post(format!("{}/api/workflows/{}/start", api_url, id))
        .send()
        .await?;

    let instance: serde_json::Value = response.json().await?;

    println!("\n{} Workflow started:\n", "✅");
    println!("  Instance ID: {}", instance["id"]);
    println!("  Status:      {}", instance["status"]);

    Ok(())
}

async fn get_workflow_status(api_url: &str, id: &str) -> CommandResult {
    let response = reqwest::get(format!("{}/api/workflows/instances/{}", api_url, id)).await?;
    let instance: serde_json::Value = response.json().await?;

    println!("\n{} Workflow Instance:\n", "ℹ️");
    println!("  ID:      {}", instance["id"]);
    println!("  Status:  {}", instance["status"]);
    println!("  Progress: {}%", (instance["progress"].as_f64().unwrap_or(0.0) * 100.0) as i32);
    println!("  Started: {}", instance["startedAt"]);

    Ok(())
}
