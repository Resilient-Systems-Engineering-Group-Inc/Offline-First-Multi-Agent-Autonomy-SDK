//! Demonstration of secrets management for offline‑first multi‑agent systems.

use secrets_management::{
    SecretsManager, InMemoryBackend, Secret, SecretQuery, AccessPolicy,
    rotation::RotationScheduler, rotation_functions,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Secrets Management Demo ===");
    
    // 1. Create a secrets manager with in‑memory backend
    let manager = SecretsManager::in_memory().await?;
    println!("Created secrets manager with in‑memory backend");
    
    // 2. Store some secrets
    let db_secret = Secret::new(
        "database_password",
        "supersecret123",
        vec!["database".to_string(), "production".to_string()],
    );
    
    let api_secret = Secret::new(
        "api_key",
        "ak_1234567890abcdef",
        vec!["api".to_string(), "external".to_string()],
    );
    
    let ssh_secret = Secret::new(
        "ssh_private_key",
        "-----BEGIN PRIVATE KEY-----\\n...",
        vec!["ssh".to_string(), "infrastructure".to_string()],
    );
    
    manager.put(db_secret).await?;
    manager.put(api_secret).await?;
    manager.put(ssh_secret).await?;
    
    println!("Stored 3 secrets: database_password, api_key, ssh_private_key");
    
    // 3. Retrieve a secret
    let retrieved = manager.get("database_password").await?;
    println!("Retrieved secret 'database_password': value = {}", retrieved.value());
    println!("  Tags: {:?}", retrieved.tags);
    println!("  Version: {}", retrieved.metadata.version);
    println!("  Created: {}", retrieved.metadata.created_at);
    
    // 4. List secrets with query
    let query = SecretQuery {
        id_prefix: None,
        tags: vec!["database".to_string()],
        metadata: std::collections::HashMap::new(),
        include_expired: false,
        limit: Some(10),
        offset: None,
    };
    
    let database_secrets = manager.list(&query).await?;
    println!("Found {} database secrets", database_secrets.len());
    
    // 5. Add an access policy
    let policy = AccessPolicy {
        id: "admin_policy".to_string(),
        allowed_agents: vec![1, 2, 3],
        required_capabilities: vec!["admin".to_string()],
        time_window: Some((9, 17)), // 9 AM to 5 PM UTC
        max_accesses: Some(100),
        access_count: 0,
    };
    
    manager.add_policy("database_password", policy).await?;
    println!("Added access policy to 'database_password'");
    
    // 6. Rotate a secret
    println!("\n=== Secret Rotation ===");
    manager.rotate("api_key", "new_api_key_9876543210").await?;
    println!("Rotated 'api_key' to new value");
    
    let rotated = manager.get("api_key").await?;
    println!("New value: {}", rotated.value());
    println!("Rotation timestamp: {:?}", rotated.metadata.last_rotated);
    
    // 7. Check secret expiration
    println!("\n=== Secret Expiration Check ===");
    for secret in &database_secrets {
        if secret.is_expired() {
            println!("Secret '{}' has EXPIRED", secret.id);
        } else if secret.needs_rotation() {
            println!("Secret '{}' needs rotation", secret.id);
        } else {
            println!("Secret '{}' is valid", secret.id);
        }
    }
    
    // 8. Create a rotation scheduler
    println!("\n=== Rotation Scheduler ===");
    let scheduler = RotationScheduler::new(Arc::new(manager.clone()));
    
    // Add a rotation job for api_key (rotate every 30 seconds for demo)
    let job = secrets_management::rotation::RotationJob::new(
        "api_key",
        30, // seconds
        rotation_functions::random_alphanumeric,
    );
    
    scheduler.add_job(job).await;
    println!("Added rotation job for 'api_key' (every 30 seconds)");
    
    // 9. Export secrets (simulated backup)
    println!("\n=== Export/Backup ===");
    let export_query = SecretQuery {
        id_prefix: None,
        tags: vec![],
        metadata: std::collections::HashMap::new(),
        include_expired: false,
        limit: None,
        offset: None,
    };
    
    let exported = manager.export(&export_query).await?;
    println!("Exported {} secrets for backup", exported.len());
    
    // 10. Get statistics
    println!("\n=== Statistics ===");
    let stats = manager.stats().await?;
    println!("Total secrets: {}", stats.total);
    println!("Expired secrets: {}", stats.expired);
    println!("Secrets needing rotation: {}", stats.needs_rotation);
    println!("Tagged secrets: {}", stats.tagged);
    
    // 11. Delete a secret
    println!("\n=== Secret Deletion ===");
    manager.delete("ssh_private_key").await?;
    println!("Deleted 'ssh_private_key'");
    
    // Verify deletion
    match manager.exists("ssh_private_key").await {
        Ok(true) => println!("ERROR: Secret still exists!"),
        Ok(false) => println!("Successfully verified deletion"),
        Err(e) => println!("Error checking existence: {}", e),
    }
    
    // 12. Version history (simulated)
    println!("\n=== Version History ===");
    let versions = manager.versions("api_key").await?;
    println!("'api_key' has {} historical versions", versions.len());
    
    // 13. Demonstrate policy evaluation
    println!("\n=== Policy Evaluation ===");
    let can_read = manager.policy_engine.can_read("database_password").await;
    let can_write = manager.policy_engine.can_write("database_password").await;
    let can_delete = manager.policy_engine.can_delete("database_password").await;
    
    println!("Permissions for 'database_password':");
    println!("  Read: {}", can_read);
    println!("  Write: {}", can_write);
    println!("  Delete: {}", can_delete);
    
    // 14. Key rotation
    println!("\n=== Encryption Key Rotation ===");
    let new_key_id = manager.rotate_keys(secrets_management::crypto::KeyAlgorithm::Aes256Gcm).await?;
    println!("Rotated encryption key to: {}", new_key_id);
    
    println!("\n=== Demo Complete ===");
    println!("Secrets management system is ready for use in multi‑agent systems.");
    println!("Features demonstrated:");
    println!("  - Secure secret storage with encryption");
    println!("  - Fine‑grained access policies");
    println!("  - Automatic secret rotation");
    println!("  - Versioning and audit trail");
    println!("  - Backup and restore capabilities");
    println!("  - Integration with mesh transport (not shown)");
    
    Ok(())
}