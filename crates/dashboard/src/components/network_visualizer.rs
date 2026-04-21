//! Network topology visualization component using Vis.js or similar.

use crate::models::{NetworkNode, NetworkEdge};
use yew::prelude::*;
use web_sys::HtmlElement;

/// Props for NetworkVisualizer component.
#[derive(Properties, PartialEq)]
pub struct NetworkVisualizerProps {
    pub nodes: UseStateHandle<Vec<NetworkNode>>,
    pub edges: UseStateHandle<Vec<NetworkEdge>>,
}

/// Interactive network topology visualization.
#[function_component(NetworkVisualizer)]
pub fn network_visualizer(props: &NetworkVisualizerProps) -> Html {
    let node_ref = use_node_ref();

    use_effect_with(props.nodes.clone(), move |nodes| {
        let _nodes = nodes.clone();
        async move {
            // In a real implementation, this would initialize Vis.js or D3.js
            // For now, we'll render a simple SVG visualization
            log::info!("Network nodes updated: {}", nodes.len());
            ()
        }
    });

    use_effect_with(props.edges.clone(), move |edges| {
        let _edges = edges.clone();
        async move {
            log::info!("Network edges updated: {}", edges.len());
            ()
        }
    });

    html! {
        <div class="network-visualizer" ref={node_ref}>
            <svg width="100%" height="400" class="network-svg">
                <defs>
                    <marker
                        id="arrowhead"
                        markerWidth="10"
                        markerHeight="7"
                        refX="9"
                        refY="3.5"
                        orient="auto"
                    >
                        <polygon points="0 0, 10 3.5, 0 7" fill="#888" />
                    </marker>
                </defs>
                
                // Draw edges
                { for props.edges.iter().flat_map(|edge| {
                    let source_node = props.nodes.iter().find(|n| n.id == edge.source);
                    let target_node = props.nodes.iter().find(|n| n.id == edge.target);
                    
                    if let (Some(source), Some(target)) = (source_node, target_node) {
                        html! {
                            <line
                                x1={format!("{}%", source.x)}
                                y1={format!("{}%", source.y)}
                                x2={format!("{}%", target.x)}
                                y2={format!("{}%", target.y)}
                                stroke={format!("rgba(100, 150, 200, {})", edge.quality)}
                                stroke-width="2"
                                marker-end="url(#arrowhead)"
                            />
                        }
                    } else {
                        html! {}
                    }
                }) }
                
                // Draw nodes
                { for props.nodes.iter().map(|node| {
                    let color = match node.status {
                        models::AgentState::Idle => "#4caf50",
                        models::AgentState::Active => "#2196f3",
                        models::AgentState::Recharging => "#ff9800",
                        models::AgentState::Error => "#f44336",
                        models::AgentState::Offline => "#9e9e9e",
                    };
                    
                    html! {
                        <g class="network-node" transform={format!("translate({}%, {}%)", node.x, node.y)}>
                            <circle
                                r="20"
                                fill={color}
                                stroke="#fff"
                                stroke-width="2"
                                class="node-circle"
                            />
                            <text
                                text-anchor="middle"
                                dy="5"
                                fill="#fff"
                                font-size="12"
                                font-weight="bold"
                            >
                                {&node.agent_id}
                            </text>
                        </g>
                    }
                }) }
            </svg>
            
            <div class="network-legend">
                <div class="legend-item">
                    <span class="legend-color" style="background-color: #4caf50;"></span>
                    <span>{"Idle"}</span>
                </div>
                <div class="legend-item">
                    <span class="legend-color" style="background-color: #2196f3;"></span>
                    <span>{"Active"}</span>
                </div>
                <div class="legend-item">
                    <span class="legend-color" style="background-color: #ff9800;"></span>
                    <span>{"Recharging"}</span>
                </div>
                <div class="legend-item">
                    <span class="legend-color" style="background-color: #f44336;"></span>
                    <span>{"Error"}</span>
                </div>
                <div class="legend-item">
                    <span class="legend-color" style="background-color: #9e9e9e;"></span>
                    <span>{"Offline"}</span>
                </div>
            </div>
        </div>
    }
}
