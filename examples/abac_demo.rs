//! ABAC (Attribute‑Based Access Control) integration demo.
//!
//! This example shows how to use the ABAC integration crate to define
//! policies based on attributes of subjects, resources, and environment.

use std::sync::Arc;
use abac_integration::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ABAC Integration Demo ===");
    
    // Create a policy engine
    let policy_engine = Arc::new(PolicyEngine::new());
    
    // Define a policy: allow agents with role "admin" to read any task
    let mut admin_policy = Policy::new(
        "admin-read-access",
        "Allow admin agents to read tasks"
    );
    
    let admin_rule = PolicyRule::new(
        "admin-can-read",
        serde_json::json!({
            "operator": "and",
            "conditions": [
                {
                    "operator": "eq",
                    "left": "$subject.attributes.role",
                    "right": "admin"
                },
                {
                    "operator": "eq",
                    "left": "$resource.resource_type",
                    "right": "task"
                },
                {
                    "operator": "eq",
                    "left": "$action",
                    "right": "read"
                }
            ]
        }),
        "allow"
    );
    admin_policy.add_rule(admin_rule);
    
    // Add the policy to the engine
    policy_engine.add_policy(admin_policy).await?;
    println!("✓ Added admin policy");
    
    // Define another policy: deny access to sensitive tasks after business hours
    let mut time_policy = Policy::new(
        "time-restricted",
        "Deny access to sensitive tasks outside business hours"
    );
    
    let time_rule = PolicyRule::new(
        "business-hours-only",
        serde_json::json!({
            "operator": "and",
            "conditions": [
                {
                    "operator": "eq",
                    "left": "$resource.attributes.sensitivity",
                    "right": "high"
                },
                {
                    "operator": "or",
                    "conditions": [
                        {
                            "operator": "lt",
                            "left": "$environment.attributes.hour",
                            "right": 9
                        },
                        {
                            "operator": "gt",
                            "left": "$environment.attributes.hour",
                            "right": 17
                        }
                    ]
                }
            ]
        }),
        "deny"
    );
    time_policy.add_rule(time_rule);
    
    policy_engine.add_policy(time_policy).await?;
    println!("✓ Added time-based policy");
    
    // Create test subjects
    let mut admin_subject = Subject::new("agent-001", "agent");
    admin_subject.add_attribute("role", serde_json::json!("admin"));
    admin_subject.add_attribute("clearance", serde_json::json!("top-secret"));
    
    let mut user_subject = Subject::new("agent-002", "agent");
    user_subject.add_attribute("role", serde_json::json!("user"));
    user_subject.add_attribute("clearance", serde_json::json!("confidential"));
    
    // Create test resources
    let mut normal_task = Resource::new("task-001", "task");
    normal_task.add_attribute("priority", serde_json::json!("medium"));
    
    let mut sensitive_task = Resource::new("task-002", "task");
    sensitive_task.add_attribute("priority", serde_json::json!("high"));
    sensitive_task.add_attribute("sensitivity", serde_json::json!("high"));
    
    // Create environment contexts
    let mut business_hours = Environment::new();
    business_hours.add_attribute("hour", serde_json::json!(14)); // 2 PM
    business_hours.add_attribute("location", serde_json::json!("office"));
    
    let mut after_hours = Environment::new();
    after_hours.add_attribute("hour", serde_json::json!(20)); // 8 PM
    after_hours.add_attribute("location", serde_json::json!("remote"));
    
    println!("\n=== Test Cases ===");
    
    // Test 1: Admin reading normal task during business hours
    let allowed = policy_engine.evaluate(
        &admin_subject,
        &normal_task,
        "read",
        &business_hours
    ).await?;
    println!("Test 1 - Admin reads normal task (business hours): {}", 
             if allowed { "ALLOWED ✓" } else { "DENIED ✗" });
    
    // Test 2: User reading normal task during business hours
    let allowed = policy_engine.evaluate(
        &user_subject,
        &normal_task,
        "read",
        &business_hours
    ).await?;
    println!("Test 2 - User reads normal task (business hours): {}", 
             if allowed { "ALLOWED ✓" } else { "DENIED ✗" });
    
    // Test 3: Admin reading sensitive task during business hours
    let allowed = policy_engine.evaluate(
        &admin_subject,
        &sensitive_task,
        "read",
        &business_hours
    ).await?;
    println!("Test 3 - Admin reads sensitive task (business hours): {}", 
             if allowed { "ALLOWED ✓" } else { "DENIED ✗" });
    
    // Test 4: Admin reading sensitive task after hours (should be denied by time policy)
    let allowed = policy_engine.evaluate(
        &admin_subject,
        &sensitive_task,
        "read",
        &after_hours
    ).await?;
    println!("Test 4 - Admin reads sensitive task (after hours): {}", 
             if allowed { "ALLOWED ✓" } else { "DENIED ✗" });
    
    // Test 5: Admin writing to task (different action)
    let allowed = policy_engine.evaluate(
        &admin_subject,
        &normal_task,
        "write",
        &business_hours
    ).await?;
    println!("Test 5 - Admin writes to normal task: {}", 
             if allowed { "ALLOWED ✓" } else { "DENIED ✗" });
    
    // Demonstrate the AccessControlManager
    println!("\n=== Using AccessControlManager ===");
    let manager = AccessControlManager::new(policy_engine.clone());
    
    // Check access through manager
    let allowed = manager.check_access(
        &admin_subject,
        &sensitive_task,
        "read",
        &business_hours
    ).await?;
    println!("Manager check - Admin reads sensitive task: {}", 
             if allowed { "ALLOWED ✓" } else { "DENIED ✗" });
    
    println!("\n=== Demo Complete ===");
    Ok(())
}