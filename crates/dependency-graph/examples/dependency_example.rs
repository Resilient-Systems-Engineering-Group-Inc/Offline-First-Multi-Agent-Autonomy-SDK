//! Example of using the dependency graph crate.

use dependency_graph::*;
use std::sync::Arc;
use tokio::sync::mpsc;

struct CustomExecutor;

#[async_trait::async_trait]
impl NodeExecutor for CustomExecutor {
    async fn execute(&self, node_id: NodeId, metadata: serde_json::Value) -> NodeResult {
        println!("Executing node {} with metadata {:?}", node_id, metadata);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), DependencyError> {
    tracing_subscriber::fmt::init();

    // Create a graph.
    let mut graph = DependencyGraph::new();

    // Add nodes.
    let node_a = Node::new("Agent A", "agent");
    let node_b = Node::new("Agent B", "agent");
    let node_c = Node::new("Task C", "task");
    let node_d = Node::new("Resource D", "resource");

    let id_a = graph.add_node(node_a);
    let id_b = graph.add_node(node_b);
    let id_c = graph.add_node(node_c);
    let id_d = graph.add_node(node_d);

    // Add edges (dependencies).
    graph.add_edge(Edge::new(id_a, id_c, EdgeType::Hard))?;
    graph.add_edge(Edge::new(id_b, id_c, EdgeType::Soft))?;
    graph.add_edge(Edge::new(id_c, id_d, EdgeType::Data))?;

    println!("Graph has {} nodes and {} edges", graph.nodes().len(), graph.edges().len());

    // Topological sort.
    let order = graph.topological_sort()?;
    println!("Topological order: {:?}", order);

    // Create scheduler.
    let executor = Arc::new(CustomExecutor);
    let config = SchedulerConfig::default();
    let (scheduler, mut event_rx) = DependencyScheduler::new(graph, executor, config);

    // Spawn event listener.
    let handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            println!("Event: {}", event.description());
        }
    });

    // Run scheduler.
    scheduler.start().await?;

    // Wait for events to finish.
    handle.await.unwrap();

    Ok(())
}