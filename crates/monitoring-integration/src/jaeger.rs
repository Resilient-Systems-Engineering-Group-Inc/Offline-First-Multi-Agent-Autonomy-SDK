//! Jaeger (OpenTelemetry) tracing integration.

use crate::error::{MonitoringError, Result};
use opentelemetry::{
    global,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, RandomIdGenerator, Sampler, Tracer},
        Resource,
    },
    trace::{TraceContextExt, TraceError, Tracer as _},
    KeyValue,
};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use std::time::Duration;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

/// Configuration for Jaeger/OTLP exporter.
#[derive(Debug, Clone)]
pub struct JaegerConfig {
    /// OTLP endpoint (e.g., "http://localhost:4317").
    pub endpoint: String,
    /// Service name.
    pub service_name: String,
    /// Sampling rate (0.0 to 1.0).
    pub sampling_rate: f64,
    /// Timeout for export.
    pub timeout_secs: u64,
    /// Protocol (grpc or http/protobuf).
    pub protocol: Protocol,
}

impl Default for JaegerConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            service_name: "offline-first-agent".to_string(),
            sampling_rate: 1.0,
            timeout_secs: 5,
            protocol: Protocol::Grpc,
        }
    }
}

/// Jaeger tracer manager.
pub struct JaegerTracer {
    config: JaegerConfig,
    tracer: Option<Tracer>,
}

impl JaegerTracer {
    /// Create a new Jaeger tracer with the given configuration.
    pub fn new(config: JaegerConfig) -> Self {
        Self {
            config,
            tracer: None,
        }
    }

    /// Initialize the tracer and install it as the global tracer.
    pub fn init(&mut self) -> Result<()> {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(&self.config.endpoint)
            .with_timeout(Duration::from_secs(self.config.timeout_secs));

        // Adjust for protocol
        match self.config.protocol {
            Protocol::Grpc => {
                // already tonic
            }
            Protocol::HttpBinary => {
                exporter = opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(&self.config.endpoint)
                    .with_timeout(Duration::from_secs(self.config.timeout_secs));
            }
            Protocol::HttpJson => {
                return Err(MonitoringError::Config(
                    "HTTP/JSON protocol not supported".to_string(),
                ));
            }
        }

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                        self.config.sampling_rate,
                    ))))
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(Resource::new(vec![KeyValue::new(
                        "service.name",
                        self.config.service_name.clone(),
                    )])),
            )
            .install_batch(opentelemetry::runtime::Tokio)
            .map_err(|e| MonitoringError::OpenTelemetry(e))?;

        self.tracer = Some(tracer);

        // Create a tracing layer that uses the OTel tracer
        let telemetry_layer = OpenTelemetryLayer::new(self.tracer.as_ref().unwrap().clone());
        let subscriber = Registry::default().with(telemetry_layer);
        tracing_subscriber::set_global_default(subscriber)
            .map_err(|e| MonitoringError::Other(format!("Failed to set global subscriber: {}", e)))?;

        log::info!(
            "Jaeger tracing initialized (endpoint: {}, service: {})",
            self.config.endpoint,
            self.config.service_name
        );
        Ok(())
    }

    /// Get the underlying OpenTelemetry tracer.
    pub fn tracer(&self) -> Option<&Tracer> {
        self.tracer.as_ref()
    }

    /// Create a span with given name and attributes.
    pub fn span(&self, name: &str, attributes: Vec<KeyValue>) -> opentelemetry::trace::Span {
        let tracer = self.tracer.as_ref().expect("Tracer not initialized");
        let span = tracer.start(name);
        span.set_attributes(attributes);
        span
    }

    /// Shutdown the tracer and flush remaining spans.
    pub fn shutdown(&self) {
        global::shutdown_tracer_provider();
        log::info!("Jaeger tracer shutdown");
    }
}

/// Convenience function to start a span using the global tracer.
pub fn start_span(name: &str) -> opentelemetry::trace::Span {
    global::tracer("offline-first").start(name)
}

/// Record an event within the current span.
pub fn record_event(name: &str, attributes: Vec<KeyValue>) {
    let span = opentelemetry::global::get_tracer_provider()
        .versioned_tracer("offline-first")
        .start(name);
    span.add_event(name, attributes);
}

/// Set a baggage item (key-value) on the current span.
pub fn set_baggage(key: &str, value: &str) {
    let cx = opentelemetry::global::get_text_map_propagator(|prop| prop.extract(&opentelemetry::Context::current()));
    let mut baggage = cx.baggage();
    baggage.insert(key.to_string(), value.to_string());
}

/// Helper to extract trace context from headers (for distributed tracing).
pub fn extract_trace_context(headers: &impl opentelemetry::propagation::Extractor) -> opentelemetry::Context {
    global::get_text_map_propagator(|prop| prop.extract(headers))
}