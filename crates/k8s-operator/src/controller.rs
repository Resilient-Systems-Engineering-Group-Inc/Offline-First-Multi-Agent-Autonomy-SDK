//! Main controller for the Kubernetes operator.

use crate::crd::{Agent, Task};
use crate::error::Error;
use crate::reconciler::{self, Context};
use kube::runtime::controller::Controller;
use kube::runtime::watcher::Config;
use kube::{Api, Client, CustomResourceExt};
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Run the operator.
pub async fn run(namespace: Option<String>) -> Result<(), Error> {
    // Initialize logging.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Kubernetes operator for Offline‑First Multi‑Agent Autonomy SDK");

    let client = Client::try_default().await?;
    let namespace = namespace.unwrap_or_else(|| "default".to_string());

    // Create CRDs if they don't exist (in a real operator you'd use `kube::runtime::wait::await_condition`).
    info!("Applying CRDs...");
    let crd_api: Api<kube::api::ApiResource> = Api::all(client.clone());
    let agent_crd = Agent::crd();
    let task_crd = Task::crd();
    // In a production operator you would use server‑side apply or a separate installation step.
    // For simplicity we just log.
    info!("Agent CRD: {}", serde_json::to_string_pretty(&agent_crd).unwrap());
    info!("Task CRD: {}", serde_json::to_string_pretty(&task_crd).unwrap());

    // Create context.
    let ctx = Arc::new(Context {
        client: client.clone(),
        namespace: namespace.clone(),
    });

    // Create controllers for Agent and Task resources.
    let agent_api = Api::<Agent>::namespaced(client.clone(), &namespace);
    let task_api = Api::<Task>::namespaced(client.clone(), &namespace);

    let agent_controller = Controller::new(agent_api, Config::default())
        .shutdown_on_signal()
        .run(reconciler::reconcile_agent, reconciler::error_policy, ctx.clone())
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Agent reconciliation succeeded: {:?}", o),
                Err(e) => warn!("Agent reconciliation failed: {:?}", e),
            }
        });

    let task_controller = Controller::new(task_api, Config::default())
        .shutdown_on_signal()
        .run(reconciler::reconcile_task, reconciler::error_policy, ctx.clone())
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Task reconciliation succeeded: {:?}", o),
                Err(e) => warn!("Task reconciliation failed: {:?}", e),
            }
        });

    // Run both controllers concurrently.
    tokio::select! {
        _ = agent_controller => info!("Agent controller stopped"),
        _ = task_controller => info!("Task controller stopped"),
    }

    Ok(())
}