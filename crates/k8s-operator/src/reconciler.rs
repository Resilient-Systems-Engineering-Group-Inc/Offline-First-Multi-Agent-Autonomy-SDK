//! Reconciliation logic for custom resources.

use crate::crd::{Agent, Task};
use crate::error::Error;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::PatchParams;
use kube::runtime::controller::Action;
use kube::{Api, Client, ResourceExt};
use std::sync::Arc;
use tracing::{info, warn};

/// Context shared across all reconcilers.
pub struct Context {
    /// Kubernetes client.
    pub client: Client,
    /// Namespace where the operator runs.
    pub namespace: String,
}

/// Reconcile an Agent resource.
pub async fn reconcile_agent(agent: Arc<Agent>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = agent.name_any();
    let namespace = agent.namespace().unwrap_or_default();
    info!("Reconciling Agent {}/{}", namespace, name);

    // TODO: Implement actual agent lifecycle management.
    // For now, we just update the status to "Running".
    let api: Api<Agent> = Api::namespaced(ctx.client.clone(), &namespace);
    let status = crate::crd::AgentStatus {
        state: "Running".to_string(),
        message: format!("Agent {} is managed by the operator", name),
        conditions: vec![Condition {
            type_: "Ready".to_string(),
            status: "True".to_string(),
            reason: Some("Reconciled".to_string()),
            message: Some("Agent is ready".to_string()),
            last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }],
        last_updated: Some(chrono::Utc::now().to_rfc3339()),
    };

    let patch = serde_json::json!({
        "status": status
    });
    let pp = PatchParams::apply("k8s-operator").force();
    api.patch_status(&name, &pp, &kube::api::Patch::Merge(&patch))
        .await?;

    // Requeue after 30 seconds for periodic health checks.
    Ok(Action::requeue(std::time::Duration::from_secs(30)))
}

/// Reconcile a Task resource.
pub async fn reconcile_task(task: Arc<Task>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = task.name_any();
    let namespace = task.namespace().unwrap_or_default();
    info!("Reconciling Task {}/{}", namespace, name);

    // TODO: Implement actual task assignment and execution.
    // For now, we just update the status to "Pending".
    let api: Api<Task> = Api::namespaced(ctx.client.clone(), &namespace);
    let status = crate::crd::TaskStatus {
        phase: "Pending".to_string(),
        assigned_agent: None,
        result: None,
        conditions: vec![Condition {
            type_: "Scheduled".to_string(),
            status: "False".to_string(),
            reason: Some("NotAssigned".to_string()),
            message: Some("Task waiting for an available agent".to_string()),
            last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }],
        last_updated: Some(chrono::Utc::now().to_rfc3339()),
    };

    let patch = serde_json::json!({
        "status": status
    });
    let pp = PatchParams::apply("k8s-operator").force();
    api.patch_status(&name, &pp, &kube::api::Patch::Merge(&patch))
        .await?;

    // Requeue after 10 seconds to check for agent availability.
    Ok(Action::requeue(std::time::Duration::from_secs(10)))
}

/// Error policy for reconciliation failures.
pub fn error_policy(_object: Arc<Agent>, error: &Error, _ctx: Arc<Context>) -> Action {
    warn!("Reconciliation error: {}", error);
    // Retry after 5 seconds.
    Action::requeue(std::time::Duration::from_secs(5))
}