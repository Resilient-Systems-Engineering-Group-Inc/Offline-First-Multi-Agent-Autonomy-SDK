//! Task commands.

use crate::commands::CommandResult;
use crate::Commands;
use anyhow::Result;
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::json;

static SUCCESS: Emoji = Emoji("✅", "+");
static ERROR: Emoji = Emoji("❌", "-");
static INFO: Emoji = Emoji("ℹ️", "i");

pub async fn execute(cmd: Commands, api_url: &str) -> CommandResult {
    match cmd {
        Commands::Task(TaskCommands::List { status, limit }) => {
            list_tasks(api_url, status, limit).await
        }
        Commands::Task(TaskCommands::Create {
            description,
            priority,
            capabilities,
        }) => create_task(api_url, &description, priority, capabilities).await,
        Commands::Task(TaskCommands::Get { id }) => get_task(api_url, &id).await,
        Commands::Task(TaskCommands::Update {
            id,
            status,
            agent,
        }) => update_task(api_url, &id, status.as_deref(), agent.as_deref()).await,
        Commands::Task(TaskCommands::Delete { id }) => delete_task(api_url, &id).await,
        _ => Err("Unknown command".into()),
    }
}

async fn list_tasks(api_url: &str, status: Option<String>, limit: i32) -> CommandResult {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Fetching tasks...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let url = if let Some(status) = status {
        format!(
            "{}/api/tasks?status={}&limit={}",
            api_url, status, limit
        )
    } else {
        format!("{}/api/tasks?limit={}", api_url, limit)
    };

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("Failed to list tasks: {}", response.status()).into());
    }

    let tasks: serde_json::Value = response.json().await?;
    pb.finish_and_clear();

    println!(
        "\n{} Found {} task(s):\n",
        SUCCESS,
        tasks.as_array().map(|a| a.len()).unwrap_or(0)
    );

    // Print as table
    if let Some(tasks) = tasks.as_array() {
        for task in tasks {
            println!(
                "  {} {} [{}] - {} (Priority: {})",
                style(&task["id"]).cyan(),
                style(&task["status"]).yellow(),
                style(&task["description"]).white(),
                if let Some(agent) = task["assignedAgent"].as_str() {
                    format!("→ {}", agent)
                } else {
                    "→ unassigned".to_string()
                },
                task["priority"]
            );
        }
    }

    Ok(())
}

async fn create_task(
    api_url: &str,
    description: &str,
    priority: i32,
    capabilities: Option<Vec<String>>,
) -> CommandResult {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Creating task...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let payload = json!({
        "description": description,
        "priority": priority,
        "requiredCapabilities": capabilities.unwrap_or_default()
    });

    let response = reqwest::Client::new()
        .post(format!("{}/api/tasks", api_url))
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to create task: {}", response.status()).into());
    }

    let task: serde_json::Value = response.json().await?;
    pb.finish_and_clear();

    println!(
        "\n{} Task created:\n",
        SUCCESS
    );
    println!(
        "  ID:       {}",
        style(&task["id"]).cyan()
    );
    println!(
        "  Status:   {}",
        style(&task["status"]).yellow()
    );
    println!(
        "  Priority: {}",
        task["priority"]
    );
    println!(
        "  Created:  {}",
        style(&task["createdAt"]).green()
    );

    Ok(())
}

async fn get_task(api_url: &str, id: &str) -> CommandResult {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Fetching task...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let response = reqwest::get(format!("{}/api/tasks/{}", api_url, id)).await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get task: {}", response.status()).into());
    }

    let task: serde_json::Value = response.json().await?;
    pb.finish_and_clear();

    println!(
        "\n{} Task Details:\n",
        INFO
    );
    println!("  ID:          {}", task["id"]);
    println!("  Description: {}", task["description"]);
    println!("  Status:      {}", task["status"]);
    println!("  Priority:    {}", task["priority"]);
    println!(
        "  Assigned:    {}",
        if let Some(agent) = task["assignedAgent"].as_str() {
            agent.to_string()
        } else {
            "None".to_string()
        }
    );
    println!("  Created:     {}", task["createdAt"]);
    println!("  Updated:     {}", task["updatedAt"]);

    Ok(())
}

async fn update_task(
    api_url: &str,
    id: &str,
    status: Option<&str>,
    agent: Option<&str>,
) -> CommandResult {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Updating task...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut payload = serde_json::Map::new();
    if let Some(status) = status {
        payload.insert("status".to_string(), json!(status));
    }
    if let Some(agent) = agent {
        payload.insert("assignedAgent".to_string(), json!(agent));
    }

    let response = reqwest::Client::new()
        .patch(format!("{}/api/tasks/{}", api_url, id))
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to update task: {}", response.status()).into());
    }

    let task: serde_json::Value = response.json().await?;
    pb.finish_and_clear();

    println!(
        "\n{} Task updated:\n",
        SUCCESS
    );
    println!("  ID:     {}", task["id"]);
    println!("  Status: {}", task["status"]);
    if let Some(agent) = task["assignedAgent"].as_str() {
        println!("  Agent:  → {}", agent);
    }

    Ok(())
}

async fn delete_task(api_url: &str, id: &str) -> CommandResult {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Deleting task...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let response = reqwest::Client::new()
        .delete(format!("{}/api/tasks/{}", api_url, id))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to delete task: {}", response.status()).into());
    }

    pb.finish_and_clear();
    println!(
        "\n{} Task {} deleted\n",
        SUCCESS,
        style(id).cyan()
    );

    Ok(())
}
