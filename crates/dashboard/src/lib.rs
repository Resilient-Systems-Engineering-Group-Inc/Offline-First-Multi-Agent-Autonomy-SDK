//! Web dashboard for monitoring and controlling the multi‑agent system.

#![deny(missing_docs, unsafe_code)]

pub mod components;
pub mod models;
pub mod services;
pub mod utils;

use yew::prelude::*;

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