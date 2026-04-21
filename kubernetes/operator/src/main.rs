//! Kubernetes operator for the Multi-Agent SDK.

mod crd;
mod controller;

use crate::controller::AgentReconciler;
use anyhow::Result;
use kube::Client;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Starting SDK Operator...");

    // Get Kubernetes client
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".to_string());

    info!("Connected to Kubernetes cluster");
    info!("Watching namespace: {}", namespace);

    // Create controller
    let agent_controller = AgentReconciler::new(client.clone(), &namespace);

    // Run controller
    agent_controller.run().await?;

    Ok(())
}
