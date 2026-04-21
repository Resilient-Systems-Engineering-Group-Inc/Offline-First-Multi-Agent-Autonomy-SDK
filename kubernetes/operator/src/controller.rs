//! Controller logic for SDK resources.

use crate::crd::{Agent, AgentSpec};
use k8s_openapi::{
    api::core::v1::{ConfigMap, Pod, Service},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams, PostParams},
    runtime::{
        controller::{Action, Context, Controller},
        watcher::Config,
    },
    Client, Resource,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

/// Agent reconciler.
pub struct AgentReconciler {
    client: Client,
    namespace: String,
}

impl AgentReconciler {
    pub fn new(client: Client, namespace: &str) -> Self {
        Self {
            client,
            namespace: namespace.to_string(),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let agents: Api<Agent> = Api::namespaced(self.client.clone(), &self.namespace);

        let context = Context::new(Arc::new(()));

        Controller::new(agents, Config::default())
            .run(
                Self::reconcile,
                Self::error_policy,
                context,
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn reconcile(
        agent: Arc<Agent>,
        _ctx: Context<'_>,
    ) -> Result<Action, Error> {
        info!("Reconciling agent: {}", agent.name_any());

        let client = Client::try_default().await?;
        let namespace = agent.namespace().unwrap_or_else(|| "default".to_string());

        // Create deployment for agent
        Self::create_agent_deployment(&client, &agent, &namespace).await?;

        // Create service for agent
        Self::create_agent_service(&client, &agent, &namespace).await?;

        Ok(Action::requeue(Duration::from_secs(300)))
    }

    async fn create_agent_deployment(
        client: &Client,
        agent: &Agent,
        namespace: &str,
    ) -> Result<(), kube::Error> {
        let deployments: Api<k8s_openapi::api::apps::v1::Deployment> = Api::namespaced(client.clone(), namespace);

        let spec = agent.spec.clone();
        let name = agent.name_any();

        let deployment = k8s_openapi::api::apps::v1::Deployment {
            metadata: ObjectMeta {
                name: Some(format!("{}-deployment", name)),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::apps::v1::DeploymentSpec {
                replicas: Some(spec.replicas as i32),
                selector: k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector {
                    match_labels: Some(vec![("app".to_string(), name.clone())].into_iter().collect()),
                    ..Default::default()
                },
                template: k8s_openapi::api::core::v1::PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(vec![("app".to_string(), name.clone())].into_iter().collect()),
                        ..Default::default()
                    }),
                    spec: Some(k8s_openapi::api::core::v1::PodSpec {
                        containers: vec![k8s_openapi::api::core::v1::Container {
                            name: name.clone(),
                            image: Some(spec.image),
                            resources: Some(k8s_openapi::api::core::v1::ResourceRequirements {
                                limits: Some(vec![
                                    ("cpu".to_string(), spec.resources.cpu_limit.parse().unwrap()),
                                    ("memory".to_string(), spec.resources.memory_limit.parse().unwrap()),
                                ].into_iter().collect()),
                                requests: Some(vec![
                                    ("cpu".to_string(), spec.resources.cpu_request.parse().unwrap()),
                                    ("memory".to_string(), spec.resources.memory_request.parse().unwrap()),
                                ].into_iter().collect()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        let pp = PostParams::default();
        deployments.create(&pp, &deployment).await?;

        info!("Created deployment for agent: {}", name);
        Ok(())
    }

    async fn create_agent_service(
        client: &Client,
        agent: &Agent,
        namespace: &str,
    ) -> Result<(), kube::Error> {
        let services: Api<Service> = Api::namespaced(client.clone(), namespace);

        let name = agent.name_any();

        let service = Service {
            metadata: ObjectMeta {
                name: Some(format!("{}-service", name)),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: Some(Service {
                selector: Some(vec![("app".to_string(), name.clone())].into_iter().collect()),
                ports: Some(vec![k8s_openapi::api::core::v1::ServicePort {
                    port: 8080,
                    target_port: Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080)),
                    protocol: Some("TCP".to_string()),
                    ..Default::default()
                }]),
                type_: Some("ClusterIP".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let pp = PostParams::default();
        services.create(&pp, &service).await?;

        info!("Created service for agent: {}", name);
        Ok(())
    }

    fn error_policy(
        _agent: Arc<Agent>,
        error: &Error,
        _ctx: Context<'_>,
    ) -> Action {
        warn!("Reconciliation error: {}", error);
        Action::requeue(Duration::from_secs(30))
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Kube error: {0}")]
    Kube(#[from] kube::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

use std::time::Duration;
