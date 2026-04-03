//! Configuration versioning demo.
//!
//! This example shows how to use the configuration versioning system
//! to track changes, create versions, and roll back.

use std::sync::Arc;
use configuration::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Configuration Versioning Demo ===");
    
    // Create an in‑memory version storage (in production you'd use file storage)
    let storage = Arc::new(InMemoryVersionStorage::new());
    let version_manager = ConfigurationVersionManager::new(storage);
    
    // Create initial configuration
    let mut config1 = Configuration::new();
    config1.insert("server.host".to_string(), serde_json::json!("localhost"));
    config1.insert("server.port".to_string(), serde_json::json!(8080));
    config1.insert("logging.level".to_string(), serde_json::json!("info"));
    
    // Create first version
    let v1_id = version_manager.create_version(
        config1.clone(),
        Some("Initial configuration".to_string()),
        Some("admin".to_string()),
    ).await?;
    println!("✓ Created version 1: {}", v1_id);
    
    // Modify configuration
    let mut config2 = config1.clone();
    config2.insert("server.port".to_string(), serde_json::json!(9090));
    config2.insert("database.url".to_string(), serde_json::json!("postgres://localhost/db"));
    
    // Create second version
    let v2_id = version_manager.create_version(
        config2.clone(),
        Some("Added database configuration".to_string()),
        Some("admin".to_string()),
    ).await?;
    println!("✓ Created version 2: {}", v2_id);
    
    // Modify configuration again
    let mut config3 = config2.clone();
    config3.insert("logging.level".to_string(), serde_json::json!("debug"));
    config3.remove("server.host");
    
    // Create third version
    let v3_id = version_manager.create_version(
        config3.clone(),
        Some("Changed logging level and removed host".to_string()),
        Some("dev".to_string()),
    ).await?;
    println!("✓ Created version 3: {}", v3_id);
    
    // List all versions
    println!("\n=== All Versions ===");
    let versions = version_manager.list_versions(None, None).await?;
    for (i, version) in versions.iter().enumerate() {
        println!("{}. {} - {} (by {:?})", 
                 i + 1,
                 version.id,
                 version.label.as_deref().unwrap_or("(no label)"),
                 version.author.as_deref().unwrap_or("unknown"));
        println!("   Created: {}", version.created_at);
        println!("   Keys: {}", version.config.len());
    }
    
    // Get a specific version
    println!("\n=== Retrieving Version 1 ===");
    let v1 = version_manager.get_version(&v1_id).await?
        .expect("version 1 should exist");
    println!("Version 1 config: {:?}", v1.config);
    
    // Compare versions
    println!("\n=== Comparing Version 1 and Version 3 ===");
    if let Some(diff) = version_manager.compare(&v1_id, &v3_id).await? {
        println!("Diff from {} to {}:", diff.from_id, diff.to_id);
        println!("  Added: {}", diff.summary.added);
        println!("  Removed: {}", diff.summary.removed);
        println!("  Modified: {}", diff.summary.modified);
        if !diff.summary.is_empty() {
            println!("  Patch: {}", diff.patch);
        }
    } else {
        println!("No diff available");
    }
    
    // Roll back to version 2
    println!("\n=== Rolling Back to Version 2 ===");
    let rolled_back_config = version_manager.rollback(&v2_id).await?;
    println!("Rolled back to version {}", v2_id);
    println!("Config after rollback: {:?}", rolled_back_config);
    
    // Demonstrate file‑based storage
    println!("\n=== File‑Based Storage Demo ===");
    let temp_dir = tempfile::tempdir()?;
    let file_storage = Arc::new(FileVersionStorage::new(temp_dir.path())?);
    let file_version_manager = ConfigurationVersionManager::new(file_storage);
    
    let test_config = Configuration::new();
    let file_version_id = file_version_manager.create_version(
        test_config,
        Some("Test version in files".to_string()),
        None,
    ).await?;
    println!("Created file‑based version: {}", file_version_id);
    
    let retrieved = file_version_manager.get_version(&file_version_id).await?;
    println!("Retrieved from file storage: {}", retrieved.is_some());
    
    println!("\n=== Demo Complete ===");
    Ok(())
}