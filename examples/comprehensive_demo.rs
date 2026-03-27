//! Comprehensive demonstration of the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This example brings together:
//! - Mesh transport with multiple backends (in‑memory)
//! - State synchronization via CRDT map
//! - Distributed consensus for task assignment
//! - Planning algorithms (deadline‑aware, dependency‑aware)
//! - Swarm simulation with failures and network delays
//! - Real‑time monitoring via Prometheus metrics and web dashboard
//!
//! Run with:
//!   cargo run --example comprehensive_demo

use std::time::Duration;
use tokio::time;
use anyhow::Result;

use common::metrics::{start_metrics_server, set_health_status, inc_tasks_created, inc_tasks_assigned};
use common::types::AgentId;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use state_sync::crdt_map::CrdtMap;
use bounded_consensus::{TwoPhaseBoundedConsensus, BoundedConsensusConfig, Proposal};
use distributed_planner::{Task, RoundRobinPlanner, DeadlineAwarePlanner, PlanningAlgorithm};
use swarm_simulator::{SwarmSimulator, SwarmSimulatorConfig};
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("=== Offline‑First Multi‑Agent Autonomy SDK Comprehensive Demo ===");

    // Start Prometheus metrics server on port 9090
    let metrics_addr: SocketAddr = "127.0.0.1:9090".parse()?;
    tokio::spawn(async move {
        if let Err(e) = start_metrics_server(metrics_addr).await {
            eprintln!("Metrics server error: {}", e);
        }
    });
    println!("Metrics server started at http://{}/metrics", metrics_addr);

    // Health status healthy
    set_health_status(true);

    // 1. Create a simple mesh network with three agents (in‑memory backend)
    println!("Creating mesh network with three agents...");
    let mut agents = Vec::new();
    for i in 0..3 {
        let config = MeshTransportConfig {
            local_agent_id: AgentId(i),
            backend_type: mesh_transport::BackendType::InMemory,
            ..Default::default()
        };
        let transport = MeshTransport::new(config).await?;
        agents.push((AgentId(i), transport));
    }

    // 2. Start each agent's transport
    for (id, transport) in &mut agents {
        transport.start().await?;
        println!("  Agent {} started", id.0);
    }

    // 3. Create a shared CRDT map and populate with initial state
    let mut crdt_map = CrdtMap::new();
    crdt_map.set("demo_key", "demo_value", AgentId(0));
    println!("CRDT map set with demo_key");

    // 4. Run a simple consensus round
    println!("Running a consensus round...");
    let consensus_config = BoundedConsensusConfig {
        local_agent_id: AgentId(0),
        participants: HashSet::from([AgentId(0), AgentId(1), AgentId(2)]),
        max_rounds: 3,
        round_duration_ms: 100,
    };
    let mut consensus = TwoPhaseBoundedConsensus::<String>::new(consensus_config);
    let proposal = Proposal {
        id: 1,
        value: "consensus decision".to_string(),
        proposer: AgentId(0),
    };
    let mut rx = consensus.propose(proposal).await?;
    // Wait for outcome (simplified)
    tokio::spawn(async move {
        while let Some(outcome) = rx.recv().await {
            match outcome {
                bounded_consensus::ConsensusOutcome::Decided(value) => {
                    println!("Consensus decided: {}", value);
                }
                _ => {}
            }
        }
    });

    // 5. Create tasks and run planning algorithms
    println!("Creating tasks and running planners...");
    let tasks = vec![
        Task {
            id: "task_1".to_string(),
            description: "Navigate to point A".to_string(),
            required_resources: vec!["cpu".to_string()],
            required_capabilities: vec!["navigation".to_string()],
            estimated_duration_secs: 30,
            deadline: Some(2000),
            priority: 5,
            dependencies: Vec::new(),
        },
        Task {
            id: "task_2".to_string(),
            description: "Take picture".to_string(),
            required_resources: vec!["camera".to_string()],
            required_capabilities: vec!["sensing".to_string()],
            estimated_duration_secs: 10,
            deadline: Some(1500),
            priority: 3,
            dependencies: vec!["task_1".to_string()],
        },
    ];
    for task in &tasks {
        inc_tasks_created();
    }

    let agents_set: HashSet<AgentId> = agents.iter().map(|(id, _)| *id).collect();
    let planner = RoundRobinPlanner;
    let assignments = planner.plan(tasks.clone(), agents_set.clone(), Vec::new()).await?;
    println!("Round‑robin planner assigned {} tasks", assignments.len());
    for assignment in &assignments {
        inc_tasks_assigned();
        println!("  Task {} -> Agent {}", assignment.task_id, assignment.agent_id.0);
    }

    // 6. Run a swarm simulation in parallel
    println!("Starting swarm simulation...");
    let sim_config = SwarmSimulatorConfig {
        num_agents: 5,
        simulation_duration_secs: 10,
        enable_logging: true,
        ..Default::default()
    };
    let mut simulator = SwarmSimulator::new(sim_config).await?;
    // Add some tasks to the simulator
    for task in tasks {
        simulator.add_task(task);
    }
    let sim_handle = tokio::spawn(async move {
        if let Err(e) = simulator.run().await {
            eprintln!("Simulation error: {}", e);
        }
    });

    // 7. Start a simple web monitor (optional)
    println!("Web monitor available at http://127.0.0.1:3030 (if web_monitor example is run separately)");

    // 8. Keep the demo running for a while, updating metrics
    println!("Demo running for 30 seconds...");
    for i in 1..=30 {
        time::sleep(Duration::from_secs(1)).await;
        if i % 5 == 0 {
            println!("  {} seconds elapsed", i);
        }
        // Simulate health check
        set_health_status(i < 25);
    }

    // 9. Cleanup
    println!("Shutting down...");
    sim_handle.abort();
    for (id, transport) in &mut agents {
        transport.stop().await?;
        println!("  Agent {} stopped", id.0);
    }
    set_health_status(false);
    println!("Demo completed successfully.");
    Ok(())
}