//! System commands.

use crate::commands::CommandResult;
use crate::Commands;
use anyhow::Result;
use console::{style, Emoji};
use serde_json::json;

static SUCCESS: Emoji = Emoji("✅", "+");

pub async fn execute(cmd: Commands, api_url: &str) -> CommandResult {
    match cmd {
        Commands::System(SystemCommands::Health) => health_check(api_url).await,
        Commands::System(SystemCommands::Metrics) => get_metrics(api_url).await,
        Commands::System(SystemCommands::Stats) => get_stats(api_url).await,
        Commands::System(SystemCommands::Interactive) => interactive_mode(api_url).await,
        _ => Err("Unknown command".into()),
    }
}

async fn health_check(api_url: &str) -> CommandResult {
    let response = reqwest::get(format!("{}/api/health", api_url)).await?;
    let health: serde_json::Value = response.json().await?;

    println!(
        "\n{} Health Check:\n",
        if health["status"] == "ok" { SUCCESS } else { "❌".into() }
    );
    println!("  Status:   {}", health["status"]);
    println!("  Version:  {}", health["version"]);
    println!("  Timestamp: {}", health["timestamp"]);

    Ok(())
}

async fn get_metrics(api_url: &str) -> CommandResult {
    let response = reqwest::get(format!("{}/api/metrics", api_url)).await?;
    let metrics: serde_json::Value = response.json().await?;

    println!("\n{} System Metrics:\n", "📊");
    println!("  Total Agents:     {}", metrics["totalAgents"]);
    println!("  Active Agents:    {}", metrics["activeAgents"]);
    println!("  Total Tasks:      {}", metrics["totalTasks"]);
    println!("  Completed Tasks:  {}", metrics["completedTasks"]);
    println!("  Failed Tasks:     {}", metrics["failedTasks"]);
    println!("  Pending Tasks:    {}", metrics["pendingTasks"]);
    println!("  Network Latency:  {:.2} ms", metrics["networkLatencyMs"]);
    println!("  Message Rate:     {:.2} msg/s", metrics["messageRate"]);

    Ok(())
}

async fn get_stats(api_url: &str) -> CommandResult {
    let response = reqwest::get(format!("{}/api/stats", api_url)).await?;
    let stats: serde_json::Value = response.json().await?;

    println!("\n{} System Statistics:\n", "📈");
    println!("  Uptime:           {}", stats["uptime"]);
    println!("  Total Requests:   {}", stats["totalRequests"]);
    println!("  Avg Response Time: {:.2} ms", stats["avgResponseTimeMs"]);
    println!("  Active Connections: {}", stats["activeConnections"]);
    println!("  Memory Usage:     {} MB", stats["memoryUsageMb"]);
    println!("  CPU Usage:        {:.1}%", stats["cpuUsagePercent"]);

    Ok(())
}

async fn interactive_mode(api_url: &str) -> CommandResult {
    use dialoguer::{Input, Select};

    println!("\n{} Interactive Mode - Type 'quit' to exit\n", "🎮");

    loop {
        print!("sdk> ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            println!("\nGoodbye!");
            break;
        }

        // Parse command
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "health" => health_check(api_url).await?,
            "metrics" => get_metrics(api_url).await?,
            "stats" => get_stats(api_url).await?,
            "tasks" => {
                let url = format!("{}/api/tasks?limit=10", api_url);
                let response = reqwest::get(&url).await?;
                let tasks: serde_json::Value = response.json().await?;
                println!("\n{} Tasks:", "📋");
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            }
            "agents" => {
                let url = format!("{}/api/agents", api_url);
                let response = reqwest::get(&url).await?;
                let agents: serde_json::Value = response.json().await?;
                println!("\n{} Agents:", "🤖");
                println!("{}", serde_json::to_string_pretty(&agents)?);
            }
            _ => {
                println!("Unknown command. Available: health, metrics, stats, tasks, agents, quit");
            }
        }
    }

    Ok(())
}
