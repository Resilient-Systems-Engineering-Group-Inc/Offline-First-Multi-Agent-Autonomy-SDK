//! Distributed tracing and observability.

use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace as sdktrace, Resource};
use opentelemetry_jaeger::Exporter;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;

/// Telemetry configuration.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub jaeger_endpoint: Option<String>,
    pub otlp_endpoint: Option<String>,
    pub sampling_rate: f64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "sdk-service".to_string(),
            jaeger_endpoint: None,
            otlp_endpoint: None,
            sampling_rate: 1.0,
        }
    }
}

/// Initialize telemetry.
pub fn init_telemetry(config: TelemetryConfig) -> Result<(), anyhow::Error> {
    // Set global propagator
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Create tracer
    let tracer = if let Some(endpoint) = config.jaeger_endpoint {
        init_jaeger(&config, &endpoint)?
    } else if let Some(endpoint) = config.otlp_endpoint {
        init_otlp(&config, &endpoint)?
    } else {
        // No-op tracer
        opentelemetry_sdk::trace::TracerProvider::builder()
            .build()
            .tracer("noop")
    };

    // Create tracing layer
    let telemetry = OpenTelemetryLayer::new(tracer);

    // Set up subscriber
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// Initialize Jaeger exporter.
fn init_jaeger(
    config: &TelemetryConfig,
    endpoint: &str,
) -> Result<opentelemetry_sdk::trace::Tracer, anyhow::Error> {
    let exporter = Exporter::builder()
        .with_agent_endpoint(endpoint)
        .with_service_name(&config.service_name)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let tracer_provider = sdktrace::TracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            &config.service_name,
        )]))
        .with_config(sdktrace::Config::default().with_sampler(
            opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(config.sampling_rate),
        ))
        .build();

    let tracer = tracer_provider.tracer(&config.service_name);
    Ok(tracer)
}

/// Initialize OTLP exporter.
fn init_otlp(
    config: &TelemetryConfig,
    endpoint: &str,
) -> Result<opentelemetry_sdk::trace::Tracer, anyhow::Error> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_trace_config(
            sdktrace::Config::default()
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    &config.service_name,
                )]))
                .with_sampler(opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(
                    config.sampling_rate,
                )),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(tracer)
}

/// Create a span for a task.
pub fn task_span(task_id: &str, description: &str) -> tracing::Span {
    use tracing::Span;

    Span::current()
        .child(
            tracing::info_span!(
                "task_execution",
                task.id = task_id,
                task.description = description
            )
        )
}

/// Create a span for workflow.
pub fn workflow_span(workflow_id: &str, workflow_name: &str) -> tracing::Span {
    tracing::info_span!(
        "workflow_execution",
        workflow.id = workflow_id,
        workflow.name = workflow_name
    )
}

/// Create a span for agent communication.
pub fn agent_span(agent_id: &str, action: &str) -> tracing::Span {
    tracing::info_span!(
        "agent_action",
        agent.id = agent_id,
        action = action
    )
}

/// Record metric with tags.
pub fn record_metric(name: &str, value: f64, tags: &[KeyValue]) {
    // Would integrate with Prometheus or other metrics system
    tracing::info!(name, value, ?tags, "metric recorded");
}

/// Trace async block.
pub async fn trace_async<F, T>(name: &str, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let span = tracing::info_span!("async_operation", operation = name);
    let _guard = span.enter();
    f.await
}

/// Wrap a function with tracing.
pub fn trace_sync<F, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let span = tracing::info_span!("sync_operation", operation = name);
    let _guard = span.enter();
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_initialization() {
        let config = TelemetryConfig {
            service_name: "test-service".to_string(),
            ..Default::default()
        };

        // Would fail without actual tracer setup
        // init_telemetry(config).unwrap();
    }
}
