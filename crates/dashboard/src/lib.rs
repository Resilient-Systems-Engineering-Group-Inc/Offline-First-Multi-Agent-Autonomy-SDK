//! Web dashboard for monitoring and controlling the multi‑agent system.
//!
//! Provides:
//! - Real-time monitoring via WebSocket
//! - REST API for control and queries
//! - Prometheus metrics export
//! - Web UI built with Yew (WASM)

#![deny(missing_docs, unsafe_code)]

#[cfg(target_arch = "wasm32")]
pub mod components;
#[cfg(target_arch = "wasm32")]
pub mod models;
#[cfg(target_arch = "wasm32")]
pub mod services;
#[cfg(target_arch = "wasm32")]
pub mod utils;

#[cfg(not(target_arch = "wasm32"))]
pub mod api;
#[cfg(not(target_arch = "wasm32"))]
pub mod websocket;
#[cfg(not(target_arch = "wasm32"))]
pub mod metrics;

#[cfg(target_arch = "wasm32")]
use yew::prelude::*;

#[cfg(target_arch = "wasm32")]
/// Root component of the dashboard.
#[function_component(App)]
pub fn app() -> Html {
    html! {
        <div class="dashboard">
            <h1>{"Offline‑First Multi‑Agent Autonomy SDK Dashboard"}</h1>
            <components::AgentList />
            <components::TaskList />
            <components::NetworkGraph />
            <components::MetricsPanel />
        </div>
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use api::{ApiState, routes};
#[cfg(not(target_arch = "wasm32"))]
pub use websocket::{WebSocketManager, WsMessage};
#[cfg(not(target_arch = "wasm32"))]
pub use metrics::MetricsCollector;

/// Start the dashboard server.
#[cfg(not(target_arch = "wasm32"))]
pub async fn start_dashboard(
    bind_address: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use warp::Filter;
    
    let state = ApiState::new();
    let routes = routes(state);
    
    tracing::info!("Starting dashboard on {}", bind_address);
    warp::serve(routes)
        .run(bind_address.parse()?)
        .await;
    
    Ok(())
}