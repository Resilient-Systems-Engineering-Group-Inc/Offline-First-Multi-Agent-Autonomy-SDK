//! Main entry point for the Kubernetes operator.

use k8s_operator::controller;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments (simple).
    let namespace = std::env::var("NAMESPACE").ok();

    info!("Starting Kubernetes operator...");
    controller::run(namespace).await?;
    info!("Operator stopped.");
    Ok(())
}