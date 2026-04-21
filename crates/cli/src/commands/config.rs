//! Configuration commands.

use crate::commands::CommandResult;
use crate::Commands;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub async fn execute(cmd: Commands) -> CommandResult {
    match cmd {
        Commands::Config(ConfigCommands::Show) => show_config().await,
        Commands::Config(ConfigCommands::Set { key, value }) => set_config(&key, &value).await,
        Commands::Config(ConfigCommands::Reset) => reset_config().await,
        _ => Err("Unknown command".into()),
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sdk")
        .join("config.json")
}

async fn show_config() -> CommandResult {
    let path = config_path();
    
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;
        
        println!("\n📋 Current Configuration:\n");
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        println!("\nℹ️  No configuration found. Using defaults.\n");
    }

    Ok(())
}

async fn set_config(key: &str, value: &str) -> CommandResult {
    let path = config_path();
    
    // Load existing config or create new
    let mut config = if path.exists() {
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Set value (simple nested support)
    let mut current = &mut config;
    let parts: Vec<&str> = key.split('.').collect();
    
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set value
            current[part] = serde_json::from_str(value).unwrap_or_else(|_| json!(value));
        } else {
            // Navigate/create nested object
            if !current.has_key(part) {
                current[part] = serde_json::json!({});
            }
            current = &mut current[part];
        }
    }

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write config
    fs::write(&path, serde_json::to_string_pretty(&config)?)?;

    println!("\n✅ Configuration updated:\n  {} = {}\n", key, value);

    Ok(())
}

async fn reset_config() -> CommandResult {
    let path = config_path();
    
    if path.exists() {
        fs::remove_file(&path)?;
        println!("\n✅ Configuration reset to defaults\n");
    } else {
        println!("\nℹ️  No configuration to reset\n");
    }

    Ok(())
}
