//! Distributed tracing with OpenTelemetry.

use opentelemetry::global;
use opentelemetry::trace::{Tracer, TracerProvider};
use opentelemetry::sdk::trace as sdktrace;
use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};
use common::types::AgentId;
use anyhow::Result;

static INITIALIZED: std::sync::Once = std::sync::Once::new();

/// Initialize distributed tracing.
pub fn init(service_name: &str, agent_id: AgentId) -> Result<()> {
    INITIALIZED.call_once(|| {
        // Set up an OTLP exporter (to Jaeger, Zipkin, etc.)
        // For simplicity, we'll use stdout for now.
        let exporter = opentelemetry_stdout::SpanExporter::default();
        let provider = sdktrace::TracerProvider::builder()
            .with_simple_exporter(exporter)
            .with_resource(Resource::new(vec![
                KeyValue::new("service.name", service_name.to_string()),
                KeyValue::new("agent.id", agent_id.0.to_string()),
            ]))
            .build();
        global::set_tracer_provider(provider.clone());

        let tracer = provider.tracer("profiling");
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        // Logging layer
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true);

        Registry::default()
            .with(filter)
            .with(fmt_layer)
            .with(telemetry)
            .init();

        tracing::info!("Tracing initialized for {} (agent {:?})", service_name, agent_id);
    });
    Ok(())
}

/// Record a custom event with metadata.
pub fn record_event(event: &str, metadata: &[(&str, &str)]) {
    let mut span = tracing::info_span!("event", name = event);
    for (key, value) in metadata {
        span.record(*key, *value);
    }
    span.in_scope(|| {
        tracing::info!(event);
    });
}

/// Start a new trace span.
pub fn start_span(name: &'static str) -> tracing::Span {
    tracing::info_span!(name)
}

/// Inject trace context into a carrier (e.g., HTTP headers).
pub fn inject_context(carrier: &mut dyn opentelemetry::propagation::Injector) {
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&global::get_text_map_propagator(|p| p.extract(&opentelemetry::Context::current())), carrier);
    });
}

/// Extract trace context from a carrier.
pub fn extract_context(carrier: &dyn opentelemetry::propagation::Extractor) -> opentelemetry::Context {
    global::get_text_map_propagator(|propagator| propagator.extract(carrier))
}