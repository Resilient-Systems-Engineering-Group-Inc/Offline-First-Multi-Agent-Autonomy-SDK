//! Metrics charts and graphs components.

use crate::models::Metrics;
use yew::prelude::*;

/// Props for MetricsCharts.
#[derive(Properties, PartialEq)]
pub struct MetricsChartsProps {
    pub metrics: UseStateHandle<Vec<Metrics>>,
    pub time_range: String,
}

/// Time-series charts for metrics.
#[function_component(MetricsCharts)]
pub fn metrics_charts(props: &MetricsChartsProps) -> Html {
    html! {
        <div class="metrics-charts">
            <div class="chart-header">
                <h2>{"Performance Metrics"}</h2>
                <select class="time-range-selector" value={props.time_range.clone()}>
                    <option value="1m">{ "Last minute" }</option>
                    <option value="5m">{ "Last 5 minutes" }</option>
                    <option value="15m">{ "Last 15 minutes" }</option>
                    <option value="1h">{ "Last hour" }</option>
                    <option value="24h">{ "Last 24 hours" }</option>
                </select>
            </div>
            
            <div class="charts-grid">
                <div class="chart-card">
                    <h3>{"Task Completion Rate"}</h3>
                    <div class="chart-placeholder">
                        { render_line_chart(props.metrics.clone(), "tasks_completed") }
                    </div>
                </div>
                
                <div class="chart-card">
                    <h3>{"Network Latency"}</h3>
                    <div class="chart-placeholder">
                        { render_line_chart(props.metrics.clone(), "network_latency") }
                    </div>
                </div>
                
                <div class="chart-card">
                    <h3>{"Message Throughput"}</h3>
                    <div class="chart-placeholder">
                        { render_bar_chart(props.metrics.clone(), "message_rate") }
                    </div>
                </div>
                
                <div class="chart-card">
                    <h3>{"Agent Resource Usage"}</h3>
                    <div class="chart-placeholder">
                        { render_area_chart(props.metrics.clone(), "resource_usage") }
                    </div>
                </div>
            </div>
            
            <div class="consensus-panel">
                <h3>{"Consensus Metrics"}</h3>
                <div class="consensus-stats">
                    <div class="stat-item">
                        <span class="stat-label">{ "Consensus Rounds:" }</span>
                        <span class="stat-value">
                            { props.metrics.iter().last().map(|m| m.consensus_rounds).unwrap_or(0) }
                        </span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">{ "Avg Round Time:" }</span>
                        <span class="stat-value">
                            { format!(
                                "{:.1} ms",
                                props.metrics.iter().last().map(|m| m.avg_consensus_time_ms).unwrap_or(0.0)
                            ) }
                        </span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">{ "Success Rate:" }</span>
                        <span class="stat-value">
                            { format!(
                                "{:.1}%",
                                props.metrics.iter().last().map(|m| m.consensus_success_rate).unwrap_or(100.0)
                            ) }
                        </span>
                    </div>
                </div>
            </div>
        </div>
    }
}

// Helper functions for chart rendering (placeholders)
fn render_line_chart(_metrics: UseStateHandle<Vec<Metrics>>, _chart_type: &str) -> Html {
    html! {
        <div class="line-chart">
            <svg width="100%" height="200" class="chart-svg">
                <text x="10" y="20" font-size="12" fill="#666">
                    { "Line chart visualization (Chart.js integration)" }
                </text>
            </svg>
        </div>
    }
}

fn render_bar_chart(_metrics: UseStateHandle<Vec<Metrics>>, _chart_type: &str) -> Html {
    html! {
        <div class="bar-chart">
            <svg width="100%" height="200" class="chart-svg">
                <text x="10" y="20" font-size="12" fill="#666">
                    { "Bar chart visualization" }
                </text>
            </svg>
        </div>
    }
}

fn render_area_chart(_metrics: UseStateHandle<Vec<Metrics>>, _chart_type: &str) -> Html {
    html! {
        <div class="area-chart">
            <svg width="100%" height="200" class="chart-svg">
                <text x="10" y="20" font-size="12" fill="#666">
                    { "Area chart visualization" }
                </text>
            </svg>
        </div>
    }
}
