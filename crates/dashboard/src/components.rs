//! Yew components for the dashboard.

mod network_visualizer;
mod task_details;
mod metrics_charts;

pub use network_visualizer::NetworkVisualizer;
pub use task_details::TaskDetailsPanel;
pub use metrics_charts::MetricsCharts;

use crate::models::{Agent, Metrics, Task};
use crate::services::MockService;
use yew::prelude::*;

/// Agent list component.
#[function_component(AgentList)]
pub fn agent_list() -> Html {
    let agents = use_state(|| MockService::get_agents());

    html! {
        <div class="agent-list">
            <h2>{"Agents"}</h2>
            <table>
                <thead>
                    <tr>
                        <th>{"ID"}</th>
                        <th>{"Capabilities"}</th>
                        <th>{"State"}</th>
                        <th>{"CPU %"}</th>
                        <th>{"Memory %"}</th>
                        <th>{"Disk %"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for agents.iter().map(|agent| html! {
                        <tr key={agent.id.clone()}>
                            <td>{&agent.id}</td>
                            <td>{agent.capabilities.join(", ")}</td>
                            <td>{format!("{:?}", agent.state)}</td>
                            <td>{agent.resources.cpu_percent}</td>
                            <td>{agent.resources.memory_percent}</td>
                            <td>{agent.resources.disk_percent}</td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </div>
    }
}

/// Task list component.
#[function_component(TaskList)]
pub fn task_list() -> Html {
    let tasks = use_state(|| MockService::get_tasks());

    html! {
        <div class="task-list">
            <h2>{"Tasks"}</h2>
            <table>
                <thead>
                    <tr>
                        <th>{"ID"}</th>
                        <th>{"Description"}</th>
                        <th>{"Assigned Agent"}</th>
                        <th>{"Status"}</th>
                        <th>{"Priority"}</th>
                        <th>{"Deadline"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for tasks.iter().map(|task| html! {
                        <tr key={task.id.clone()}>
                            <td>{&task.id}</td>
                            <td>{&task.description}</td>
                            <td>{task.assigned_agent.as_deref().unwrap_or("—")}</td>
                            <td>{format!("{:?}", task.status)}</td>
                            <td>{task.priority}</td>
                            <td>{task.deadline.map(|d| d.to_string()).unwrap_or_else(|| "—".to_string())}</td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </div>
    }
}

/// Network graph component (placeholder).
#[function_component(NetworkGraph)]
pub fn network_graph() -> Html {
    html! {
        <div class="network-graph">
            <h2>{"Network Topology"}</h2>
            <div style="width: 100%; height: 300px; background: #f0f0f0; border-radius: 8px; display: flex; align-items: center; justify-content: center;">
                <p>{"Graph visualization would appear here."}</p>
            </div>
        </div>
    }
}

/// Metrics panel component.
#[function_component(MetricsPanel)]
pub fn metrics_panel() -> Html {
    let metrics = use_state(|| MockService::get_metrics());

    html! {
        <div class="metrics-panel">
            <h2>{"System Metrics"}</h2>
            <div class="metrics-grid">
                <div class="metric-card">
                    <h3>{"Total Agents"}</h3>
                    <p class="metric-value">{metrics.total_agents}</p>
                </div>
                <div class="metric-card">
                    <h3>{"Total Tasks"}</h3>
                    <p class="metric-value">{metrics.total_tasks}</p>
                </div>
                <div class="metric-card">
                    <h3>{"Tasks Completed"}</h3>
                    <p class="metric-value">{metrics.tasks_completed}</p>
                </div>
                <div class="metric-card">
                    <h3>{"Tasks Failed"}</h3>
                    <p class="metric-value">{metrics.tasks_failed}</p>
                </div>
                <div class="metric-card">
                    <h3>{"Network Latency"}</h3>
                    <p class="metric-value">{format!("{:.1} ms", metrics.network_latency_ms)}</p>
                </div>
                <div class="metric-card">
                    <h3>{"Message Rate"}</h3>
                    <p class="metric-value">{format!("{:.1} msg/s", metrics.message_rate)}</p>
                </div>
            </div>
        </div>
    }
}