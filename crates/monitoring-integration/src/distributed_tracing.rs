//! Distributed tracing for multi‑agent systems.
//!
//! This module extends OpenTelemetry/Jaeger tracing with agent‑aware context
//! propagation, mesh‑transport integration, and specialized span attributes
//! for offline‑first multi‑agent scenarios.

use opentelemetry::{
    global,
    propagation::{Extractor, Injector, TextMapPropagator},
    trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
    Context,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{MonitoringError, Result};
use crate::jaeger::JaegerTracer;

/// Agent‑aware trace context for distributed tracing across agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTraceContext {
    /// OpenTelemetry trace ID.
    pub trace_id: TraceId,
    
    /// OpenTelemetry span ID.
    pub span_id: SpanId,
    
    /// Trace flags.
    pub trace_flags: TraceFlags,
    
    /// Trace state.
    pub trace_state: TraceState,
    
    /// Agent ID that created the span.
    pub source_agent: u64,
    
    /// List of agent IDs involved in the trace.
    pub involved_agents: Vec<u64>,
    
    /// Custom attributes for agent‑specific context.
    pub attributes: HashMap<String, String>,
}

impl AgentTraceContext {
    /// Create a new agent trace context from the current OpenTelemetry context.
    pub fn from_current_context(source_agent: u64) -> Option<Self> {
        let ctx = Context::current();
        let span = ctx.span();
        let span_context = span.span_context();
        
        if !span_context.is_valid() {
            return None;
        }
        
        Some(Self {
            trace_id: span_context.trace_id(),
            span_id: span_context.span_id(),
            trace_flags: span_context.trace_flags(),
            trace_state: span_context.trace_state().clone(),
            source_agent,
            involved_agents: vec![source_agent],
            attributes: HashMap::new(),
        })
    }
    
    /// Convert to OpenTelemetry span context.
    pub fn to_span_context(&self) -> SpanContext {
        SpanContext::new(
            self.trace_id,
            self.span_id,
            self.trace_flags,
            true, // is_remote
            self.trace_state.clone(),
        )
    }
    
    /// Inject this context into a carrier (e.g., HTTP headers, mesh message).
    pub fn inject_into<T: Injector + Extractor>(&self, carrier: &mut T) {
        // Use standard W3C TraceContext propagation
        let propagator = global::get_text_map_propagator(|propagator| propagator.clone());
        
        // Create a context with our span context
        let span_context = self.to_span_context();
        let ctx = Context::current_with_span(
            opentelemetry::trace::Span::new(span_context, "agent_trace", &opentelemetry::trace::NoopTracerProvider::default())
        );
        
        propagator.inject_context(&ctx, carrier);
        
        // Also inject agent‑specific fields
        carrier.set("x-agent-id", &self.source_agent.to_string());
        carrier.set("x-involved-agents", &serde_json::to_string(&self.involved_agents).unwrap_or_default());
        
        for (key, value) in &self.attributes {
            carrier.set(&format!("x-agent-attr-{}", key), value);
        }
    }
    
    /// Extract from a carrier.
    pub fn extract_from<T: Extractor>(carrier: &T) -> Option<Self> {
        let propagator = global::get_text_map_propagator(|propagator| propagator.clone());
        let ctx = propagator.extract(carrier);
        let span = ctx.span();
        let span_context = span.span_context();
        
        if !span_context.is_valid() {
            return None;
        }
        
        let source_agent = carrier.get("x-agent-id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let involved_agents: Vec<u64> = carrier.get("x-involved-agents")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| vec![source_agent]);
        
        let mut attributes = HashMap::new();
        for key in carrier.keys() {
            if let Some(attr_key) = key.strip_prefix("x-agent-attr-") {
                if let Some(value) = carrier.get(key) {
                    attributes.insert(attr_key.to_string(), value.to_string());
                }
            }
        }
        
        Some(Self {
            trace_id: span_context.trace_id(),
            span_id: span_context.span_id(),
            trace_flags: span_context.trace_flags(),
            trace_state: span_context.trace_state().clone(),
            source_agent,
            involved_agents,
            attributes,
        })
    }
    
    /// Add an agent to the involved agents list.
    pub fn add_agent(&mut self, agent_id: u64) {
        if !self.involved_agents.contains(&agent_id) {
            self.involved_agents.push(agent_id);
        }
    }
    
    /// Set a custom attribute.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }
}

/// Carrier for mesh transport messages.
#[derive(Debug, Default)]
pub struct MeshMessageCarrier {
    headers: HashMap<String, String>,
}

impl MeshMessageCarrier {
    /// Create a new carrier from a mesh message payload.
    pub fn from_payload(payload: &[u8]) -> Result<Self> {
        // In real implementation, parse headers from payload
        Ok(Self::default())
    }
    
    /// Convert to payload with injected headers.
    pub fn to_payload(&self) -> Result<Vec<u8>> {
        // In real implementation, serialize headers into payload
        Ok(Vec::new())
    }
}

impl Injector for MeshMessageCarrier {
    fn set(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }
}

impl Extractor for MeshMessageCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }
    
    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|s| s.as_str()).collect()
    }
}

/// Distributed tracing manager for multi‑agent systems.
#[derive(Debug)]
pub struct DistributedTracingManager {
    jaeger_tracer: Option<JaegerTracer>,
    local_agent_id: u64,
    active_traces: RwLock<HashMap<TraceId, AgentTraceContext>>,
}

impl DistributedTracingManager {
    /// Create a new distributed tracing manager.
    pub fn new(local_agent_id: u64) -> Self {
        Self {
            jaeger_tracer: None,
            local_agent_id,
            active_traces: RwLock::new(HashMap::new()),
        }
    }
    
    /// Set the Jaeger tracer backend.
    pub fn with_jaeger_tracer(mut self, tracer: JaegerTracer) -> Self {
        self.jaeger_tracer = Some(tracer);
        self
    }
    
    /// Start a new distributed trace.
    pub async fn start_trace(&self, name: &str) -> Result<AgentTraceContext> {
        let tracer = self.jaeger_tracer.as_ref()
            .ok_or_else(|| MonitoringError::Config("Jaeger tracer not configured".into()))?;
        
        // Initialize if not already
        if tracer.tracer.is_none() {
            let mut tracer_mut = unsafe { &mut *(tracer as *const _ as *mut JaegerTracer) };
            tracer_mut.init()?;
        }
        
        // Create span using OpenTelemetry
        let span = global::tracer("agent-system").start(name);
        let span_context = span.span_context();
        
        let trace_context = AgentTraceContext {
            trace_id: span_context.trace_id(),
            span_id: span_context.span_id(),
            trace_flags: span_context.trace_flags(),
            trace_state: span_context.trace_state().clone(),
            source_agent: self.local_agent_id,
            involved_agents: vec![self.local_agent_id],
            attributes: HashMap::new(),
        };
        
        // Store in active traces
        let mut active_traces = self.active_traces.write().await;
        active_traces.insert(trace_context.trace_id, trace_context.clone());
        
        Ok(trace_context)
    }
    
    /// Continue a trace from an incoming context.
    pub async fn continue_trace(&self, context: &AgentTraceContext, span_name: &str) -> Result<AgentTraceContext> {
        // Add local agent to involved agents
        let mut context = context.clone();
        context.add_agent(self.local_agent_id);
        
        // Create child span
        let parent_span_context = context.to_span_context();
        let span = global::tracer("agent-system")
            .span_builder(span_name)
            .with_parent(Context::current_with_span(
                opentelemetry::trace::Span::new(parent_span_context, "parent", &opentelemetry::trace::NoopTracerProvider::default())
            ))
            .start(&global::tracer_provider());
        
        let child_context = AgentTraceContext {
            trace_id: context.trace_id,
            span_id: span.span_context().span_id(),
            trace_flags: span.span_context().trace_flags(),
            trace_state: span.span_context().trace_state().clone(),
            source_agent: self.local_agent_id,
            involved_agents: context.involved_agents,
            attributes: context.attributes,
        };
        
        // Update active traces
        let mut active_traces = self.active_traces.write().await;
        active_traces.insert(child_context.trace_id, child_context.clone());
        
        Ok(child_context)
    }
    
    /// Inject trace context into a mesh message.
    pub async fn inject_into_mesh_message(&self, trace_id: TraceId, payload: &mut Vec<u8>) -> Result<()> {
        let active_traces = self.active_traces.read().await;
        if let Some(context) = active_traces.get(&trace_id) {
            let mut carrier = MeshMessageCarrier::default();
            context.inject_into(&mut carrier);
            
            // In real implementation, merge carrier headers with payload
            // For now, we'll just log
            log::debug!("Injected trace {:?} into mesh message", trace_id);
        }
        
        Ok(())
    }
    
    /// Extract trace context from a mesh message.
    pub async fn extract_from_mesh_message(&self, payload: &[u8]) -> Result<Option<AgentTraceContext>> {
        let carrier = MeshMessageCarrier::from_payload(payload)?;
        let context = AgentTraceContext::extract_from(&carrier);
        
        if let Some(context) = &context {
            log::debug!("Extracted trace {:?} from mesh message", context.trace_id);
            
            // Store in active traces
            let mut active_traces = self.active_traces.write().await;
            active_traces.insert(context.trace_id, context.clone());
        }
        
        Ok(context)
    }
    
    /// Get active trace by ID.
    pub async fn get_trace(&self, trace_id: TraceId) -> Option<AgentTraceContext> {
        let active_traces = self.active_traces.read().await;
        active_traces.get(&trace_id).cloned()
    }
    
    /// List all active traces.
    pub async fn list_traces(&self) -> Vec<AgentTraceContext> {
        let active_traces = self.active_traces.read().await;
        active_traces.values().cloned().collect()
    }
    
    /// End a trace (remove from active traces).
    pub async fn end_trace(&self, trace_id: TraceId) {
        let mut active_traces = self.active_traces.write().await;
        active_traces.remove(&trace_id);
        log::debug!("Ended trace {:?}", trace_id);
    }
    
    /// Create a span for agent‑specific operations.
    pub fn agent_span(&self, name: &str, agent_id: u64) -> opentelemetry::trace::Span {
        let tracer = global::tracer("agent-system");
        let span = tracer.start(name);
        
        // Add agent‑specific attributes
        span.set_attribute(opentelemetry::Key::new("agent.id").i64(agent_id as i64));
        span.set_attribute(opentelemetry::Key::new("agent.local").bool(agent_id == self.local_agent_id));
        
        span
    }
    
    /// Record an agent event with tracing.
    pub fn record_agent_event(&self, event: &str, agent_id: u64, attributes: &[(&str, &str)]) {
        let span = self.agent_span(event, agent_id);
        
        for (key, value) in attributes {
            span.set_attribute(opentelemetry::Key::new(*key).string(*value));
        }
        
        span.add_event(event, vec![]);
        span.end();
    }
    
    /// Record mesh transport event with tracing.
    pub fn record_mesh_event(&self, event: &str, from_agent: u64, to_agent: Option<u64>, message_size: usize) {
        let span_name = format!("mesh.{}", event);
        let span = self.agent_span(&span_name, from_agent);
        
        span.set_attribute(opentelemetry::Key::new("mesh.event").string(event));
        span.set_attribute(opentelemetry::Key::new("mesh.from_agent").i64(from_agent as i64));
        
        if let Some(to) = to_agent {
            span.set_attribute(opentelemetry::Key::new("mesh.to_agent").i64(to as i64));
        }
        
        span.set_attribute(opentelemetry::Key::new("mesh.message_size").i64(message_size as i64));
        span.set_attribute(opentelemetry::Key::new("mesh.direction")
            .string(if to_agent.is_some() { "send" } else { "broadcast" }));
        
        span.end();
    }
    
    /// Record consensus event with tracing.
    pub fn record_consensus_event(&self, event: &str, round: u64, participants: usize, decision: &str) {
        let span = self.agent_span(&format!("consensus.{}", event), self.local_agent_id);
        
        span.set_attribute(opentelemetry::Key::new("consensus.event").string(event));
        span.set_attribute(opentelemetry::Key::new("consensus.round").i64(round as i64));
        span.set_attribute(opentelemetry::Key::new("consensus.participants").i64(participants as i64));
        span.set_attribute(opentelemetry::Key::new("consensus.decision").string(decision));
        
        span.end();
    }
}

/// Convenience macros for distributed tracing.
#[macro_export]
macro_rules! trace_agent_span {
    ($manager:expr, $name:expr, $agent_id:expr) => {{
        let span = $manager.agent_span($name, $agent_id);
        let _guard = span.enter();
        span
    }};
}

#[macro_export]
macro_rules! trace_mesh_event {
    ($manager:expr, $event:expr, $from:expr, $to:expr, $size:expr) => {
        $manager.record_mesh_event($event, $from, $to, $size);
    };
}

#[macro_export]
macro_rules! trace_consensus_event {
    ($manager:expr, $event:expr, $round:expr, $participants:expr, $decision:expr) => {
        $manager.record_consensus_event($event, $round, $participants, $decision);
    };
}

/// Initialize distributed tracing with default configuration.
pub async fn init_distributed_tracing(local_agent_id: u64, jaeger_endpoint: Option<&str>) -> Result<DistributedTracingManager> {
    let mut manager = DistributedTracingManager::new(local_agent_id);
    
    if let Some(endpoint) = jaeger_endpoint {
        let config = crate::jaeger::JaegerConfig {
            endpoint: endpoint.to_string(),
            service_name: format!("agent-{}", local_agent_id),
            sampling_rate: 1.0,
            timeout_secs: 5,
            protocol: opentelemetry_otlp::Protocol::Grpc,
        };
        
        let jaeger_tracer = crate::jaeger::JaegerTracer::new(config);
        manager = manager.with_jaeger_tracer(jaeger_tracer);
    }
    
    Ok(manager)
}