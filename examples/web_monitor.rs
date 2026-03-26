//! Web monitor for offline‑first multi‑agent autonomy SDK.
//!
//! This example runs a simple web server that displays connected agents and their state.

use warp::Filter;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;
use std::collections::HashMap;
use common::types::AgentId;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use state_sync::StateSync;
use agent_core::Agent;
use std::net::SocketAddr;

/// Shared state for the web server.
#[derive(Clone)]
struct AppState {
    /// Map from agent ID to agent info.
    agents: Arc<RwLock<HashMap<u64, AgentInfo>>>,
}

#[derive(Clone, serde::Serialize)]
struct AgentInfo {
    id: u64,
    peers: Vec<u64>,
    key_count: usize,
    last_seen: std::time::SystemTime,
}

impl AppState {
    fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn update_agent(&self, id: u64, peers: Vec<u64>, key_count: usize) {
        let mut map = self.agents.write().await;
        map.insert(id, AgentInfo {
            id,
            peers,
            key_count,
            last_seen: std::time::SystemTime::now(),
        });
    }
}

/// Start a simple agent that periodically broadcasts changes.
async fn run_agent(agent_id: u64, state: AppState) -> anyhow::Result<()> {
    let config = MeshTransportConfig {
        local_agent_id: AgentId(agent_id),
        use_in_memory: true, // Use in‑memory backend for demo
        ..Default::default()
    };
    let mut agent = Agent::new(AgentId(agent_id), config)?;
    agent.start()?;

    // Simulate some changes
    agent.set_value("demo_key", json!("demo_value"))?;

    loop {
        // Update web state
        let peers = agent.transport().peers();
        let key_count = agent.state().map().len();
        state.update_agent(agent_id, peers.iter().map(|p| p.agent_id.0).collect(), key_count).await;

        // Broadcast changes every 5 seconds
        agent.broadcast_changes().await?;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state = AppState::new();

    // Start a few demo agents in the background
    let state1 = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_agent(1, state1).await {
            eprintln!("Agent 1 error: {}", e);
        }
    });
    let state2 = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_agent(2, state2).await {
            eprintln!("Agent 2 error: {}", e);
        }
    });
    let state3 = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_agent(3, state3).await {
            eprintln!("Agent 3 error: {}", e);
        }
    });

    // Define web routes
    let state_filter = warp::any().map(move || state.clone());

    // GET /agents returns JSON list of agents
    let agents_route = warp::path("agents")
        .and(warp::get())
        .and(state_filter.clone())
        .and_then(|state: AppState| async move {
            let agents = state.agents.read().await;
            let list: Vec<AgentInfo> = agents.values().cloned().collect();
            Ok::<_, warp::Rejection>(warp::reply::json(&list))
        });

    // GET / serves a simple HTML page
    let index_route = warp::path::end()
        .and(warp::get())
        .map(|| {
            warp::reply::html(INDEX_HTML)
        });

    // Combine routes
    let routes = index_route
        .or(agents_route)
        .with(warp::cors().allow_any_origin());

    let addr: SocketAddr = "127.0.0.1:3030".parse()?;
    println!("Web monitor listening on http://{}", addr);
    warp::serve(routes).run(addr).await;

    Ok(())
}

const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Offline‑First Multi‑Agent Autonomy SDK Monitor</title>
    <style>
        body { font-family: sans-serif; margin: 2em; }
        h1 { color: #333; }
        .agent { border: 1px solid #ccc; border-radius: 5px; padding: 1em; margin: 1em 0; }
        .agent-id { font-weight: bold; font-size: 1.2em; }
        .peers { margin-left: 1em; }
        .key-count { color: green; }
        .last-seen { color: gray; font-size: 0.9em; }
    </style>
</head>
<body>
    <h1>Agent Monitor</h1>
    <div id="agents"></div>
    <script>
        async function fetchAgents() {
            const resp = await fetch('/agents');
            const agents = await resp.json();
            const container = document.getElementById('agents');
            container.innerHTML = '';
            agents.forEach(agent => {
                const div = document.createElement('div');
                div.className = 'agent';
                div.innerHTML = `
                    <div class="agent-id">Agent ${agent.id}</div>
                    <div class="peers">Peers: ${agent.peers.join(', ') || 'none'}</div>
                    <div class="key-count">Keys in CRDT: ${agent.key_count}</div>
                    <div class="last-seen">Last seen: ${new Date(agent.last_seen).toLocaleTimeString()}</div>
                `;
                container.appendChild(div);
            });
        }
        setInterval(fetchAgents, 2000);
        fetchAgents();
    </script>
</body>
</html>
"#;