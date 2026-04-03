//! Demonstration of distributed tracing for multi‑agent systems.

use monitoring_integration::{
    distributed_tracing::{DistributedTracingManager, AgentTraceContext, init_distributed_tracing},
    jaeger::JaegerConfig,
};
use opentelemetry::trace::TraceId;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Distributed Tracing Demo ===");
    
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // 1. Create a distributed tracing manager
    let local_agent_id = 42;
    let manager = init_distributed_tracing(local_agent_id, Some("http://localhost:4317")).await?;
    println!("Created distributed tracing manager for agent {}", local_agent_id);
    
    // 2. Start a new trace
    println!("\n=== Starting a New Trace ===");
    let trace_context = manager.start_trace("agent_coordination_workflow").await?;
    println!("Started trace:");
    println!("  Trace ID: {:?}", trace_context.trace_id);
    println!("  Span ID: {:?}", trace_context.span_id);
    println!("  Source agent: {}", trace_context.source_agent);
    println!("  Involved agents: {:?}", trace_context.involved_agents);
    
    // 3. Record some agent events
    println!("\n=== Recording Agent Events ===");
    manager.record_agent_event(
        "agent_initialized",
        local_agent_id,
        &[("version", "1.0.0"), ("capabilities", "planning,communication")],
    );
    println!("Recorded 'agent_initialized' event");
    
    // 4. Simulate mesh communication with tracing
    println!("\n=== Simulating Mesh Communication ===");
    manager.record_mesh_event("send", local_agent_id, Some(43), 1024);
    println!("Recorded mesh 'send' event to agent 43");
    
    manager.record_mesh_event("broadcast", local_agent_id, None, 2048);
    println!("Recorded mesh 'broadcast' event");
    
    // 5. Simulate consensus with tracing
    println!("\n=== Simulating Consensus ===");
    manager.record_consensus_event("propose", 1, 5, "accepted");
    println!("Recorded consensus 'propose' event (round 1, 5 participants)");
    
    manager.record_consensus_event("commit", 1, 5, "committed");
    println!("Recorded consensus 'commit' event");
    
    // 6. Continue trace in a different context (simulating another agent)
    println!("\n=== Continuing Trace Across Agents ===");
    
    // Simulate receiving trace context from another agent
    let mut received_context = trace_context.clone();
    received_context.source_agent = 43; // Simulate coming from agent 43
    received_context.add_agent(43);
    
    println!("Received trace context from agent 43");
    println!("  Involved agents now: {:?}", received_context.involved_agents);
    
    // Continue the trace locally
    let continued_context = manager.continue_trace(&received_context, "process_incoming_message").await?;
    println!("Continued trace with new span:");
    println!("  New Span ID: {:?}", continued_context.span_id);
    println!("  Involved agents: {:?}", continued_context.involved_agents);
    
    // 7. List active traces
    println!("\n=== Active Traces ===");
    let active_traces = manager.list_traces().await;
    println!("Number of active traces: {}", active_traces.len());
    
    for (i, trace) in active_traces.iter().enumerate() {
        println!("  Trace {}: ID={:?}, agents={:?}", i + 1, trace.trace_id, trace.involved_agents);
    }
    
    // 8. Demonstrate trace context injection/extraction (simulated)
    println!("\n=== Trace Context Propagation ===");
    
    // Create a carrier and inject trace context
    use monitoring_integration::distributed_tracing::MeshMessageCarrier;
    use opentelemetry::propagation::Injector;
    
    let mut carrier = MeshMessageCarrier::default();
    trace_context.inject_into(&mut carrier);
    
    println!("Injected trace context into carrier with {} headers", carrier.keys().len());
    
    // Extract from carrier
    let extracted = AgentTraceContext::extract_from(&carrier);
    match extracted {
        Some(ctx) => {
            println!("Successfully extracted trace context:");
            println!("  Trace ID: {:?}", ctx.trace_id);
            println!("  Source agent: {}", ctx.source_agent);
        }
        None => println!("Failed to extract trace context"),
    }
    
    // 9. Use convenience macros (if they were working)
    println!("\n=== Using Tracing Macros ===");
    
    // Note: Macros would need to be in scope
    // trace_mesh_event!(&manager, "receive", 43, Some(local_agent_id), 512);
    // println!("Used trace_mesh_event! macro");
    
    // 10. Simulate a multi‑agent workflow with tracing
    println!("\n=== Simulating Multi‑Agent Workflow ===");
    
    // Start a workflow trace
    let workflow_trace = manager.start_trace("multi_agent_task_execution").await?;
    
    // Simulate different agents participating
    let mut workflow_context = workflow_trace.clone();
    
    for agent_id in 1..=3 {
        workflow_context.source_agent = agent_id;
        workflow_context.add_agent(agent_id);
        
        manager.record_agent_event(
            "task_execution",
            agent_id,
            &[("task_id", "123"), ("status", "executing")],
        );
        
        println!("  Agent {} executed task", agent_id);
        
        // Simulate communication between agents
        if agent_id < 3 {
            manager.record_mesh_event("send", agent_id, Some(agent_id + 1), 256);
            println!("  Agent {} sent result to agent {}", agent_id, agent_id + 1);
        }
    }
    
    // Record final result
    manager.record_consensus_event("task_complete", 1, 3, "success");
    println!("  All agents completed task successfully");
    
    // 11. End the traces
    println!("\n=== Ending Traces ===");
    manager.end_trace(trace_context.trace_id).await;
    manager.end_trace(workflow_trace.trace_id).await;
    println!("Ended all traces");
    
    // 12. Demonstrate integration with other monitoring components
    println!("\n=== Integration with Monitoring Stack ===");
    println!("Distributed tracing integrates with:");
    println!("  - Jaeger/OpenTelemetry for trace collection");
    println!("  - Prometheus for metrics correlation");
    println!("  - Grafana for visualization");
    println!("  - Mesh transport for context propagation");
    println!("  - Agent lifecycle for span attribution");
    
    // 13. Show how to correlate traces with metrics
    println!("\n=== Trace‑Metric Correlation ===");
    println!("Traces can be correlated with metrics using:");
    println!("  - Trace ID in metric labels");
    println!("  - Span attributes in log entries");
    println!("  - Agent ID as common dimension");
    println!("  - Timestamp alignment");
    
    // Simulate a delay to see traces in Jaeger UI
    println!("\n=== Demo Complete ===");
    println!("If Jaeger is running at http://localhost:16686, you can view these traces.");
    println!("Waiting 2 seconds for traces to be exported...");
    sleep(Duration::from_secs(2)).await;
    
    println!("\nDistributed tracing is now fully integrated into the multi‑agent system!");
    println!("Features demonstrated:");
    println!("  - Agent‑aware trace context propagation");
    println!("  - Mesh transport integration");
    println!("  - Multi‑agent workflow tracing");
    println!("  - Consensus and communication event tracing");
    println!("  - Integration with OpenTelemetry/Jaeger");
    
    Ok(())
}