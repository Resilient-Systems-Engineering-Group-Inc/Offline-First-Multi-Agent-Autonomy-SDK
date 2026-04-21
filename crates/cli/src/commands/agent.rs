//! Agent commands.

use crate::commands::CommandResult;
use crate::Commands;
use anyhow::Result;
use console::style;

pub async fn execute(cmd: Commands, api_url: &str) -> CommandResult {
    match cmd {
        Commands::Agent(AgentCommands::List { status }) => list_agents(api_url, status).await,
        Commands::Agent(AgentCommands::Get { id }) => get_agent(api_url, &id).await,
        Commands::Agent(AgentCommands::Register { name, capabilities }) => {
            register_agent(api_url, &name, capabilities).await
        }
        Commands::Agent(AgentCommands::Unregister { id }) => {
            unregister_agent(api_url, &id).await
        }
        _ => Err("Unknown command".into()),
    }
}

async fn list_agents(api_url: &str, status: Option<String>) -> CommandResult {
    let url = if let Some(status) = status {
        format!("{}/api/agents?status={}", api_url, status)
    } else {
        format!("{}/api/agents", api_url)
    };

    let response = reqwest::get(&url).await?;
    let agents: serde_json::Value = response.json().await?;

    println!("\n{} Found {} agent(s):\n", "✅", agents.as_array().map(|a| a.len()).unwrap_or(0));

    if let Some(agents) = agents.as_array() {
        for agent in agents {
            println!(
                "  {} {} [{}] - Capabilities: {}",
                style(&agent["id"]).cyan(),
                style(&agent["status"]).yellow(),
                style(&agent["name"]).white(),
                agent["capabilities"].as_array().map(|c| c.iter()
                    .map(|cap| cap.as_str().unwrap_or("unknown"))
                    .collect::<Vec<_>>()
                    .join(", ")
                ).unwrap_or("none")
            );
        }
    }

    Ok(())
}

async fn get_agent(api_url: &str, id: &str) -> CommandResult {
    let response = reqwest::get(format!("{}/api/agents/{}", api_url, id)).await?;
    let agent: serde_json::Value = response.json().await?;

    println!("\n{} Agent Details:\n", "ℹ️");
    println!("  ID:          {}", agent["id"]);
    println!("  Name:        {}", agent["name"]);
    println!("  Status:      {}", agent["status"]);
    println!("  Capabilities: {}", agent["capabilities"].as_array().map(|c| c.iter()
        .map(|cap| cap.as_str().unwrap_or("unknown"))
        .collect::<Vec<_>>()
        .join(", ")
    ).unwrap_or("none"));
    println!("  Last Heartbeat: {}", agent["lastHeartbeat"]);

    Ok(())
}

async fn register_agent(api_url: &str, name: &str, capabilities: Vec<String>) -> CommandResult {
    let payload = serde_json::json!({
        "name": name,
        "capabilities": capabilities
    });

    let response = reqwest::Client::new()
        .post(format!("{}/api/agents", api_url))
        .json(&payload)
        .send()
        .await?;

    let agent: serde_json::Value = response.json().await?;

    println!("\n{} Agent registered:\n", "✅");
    println!("  ID:          {}", agent["id"]);
    println!("  Name:        {}", agent["name"]);
    println!("  Capabilities: {}", agent["capabilities"].as_array().map(|c| c.iter()
        .map(|cap| cap.as_str().unwrap_or("unknown"))
        .collect::<Vec<_>>()
        .join(", ")
    ).unwrap_or("none"));

    Ok(())
}

async fn unregister_agent(api_url: &str, id: &str) -> CommandResult {
    let response = reqwest::Client::new()
        .delete(format!("{}/api/agents/{}", api_url, id))
        .send()
        .await?;

    println!("\n{} Agent {} unregistered\n", "✅", id);

    Ok(())
}
