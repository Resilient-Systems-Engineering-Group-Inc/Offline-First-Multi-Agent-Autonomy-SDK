//! Dashboard server example.
//!
//! Demonstrates:
//! - Starting the dashboard server
//! - REST API endpoints
//! - WebSocket connections
//! - Prometheus metrics

use dashboard::{start_dashboard, ApiState, routes};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Dashboard Server Demo ===\n");

    // Option 1: Simple dashboard start
    info!("Option 1: Simple dashboard start");
    info!("  Call: start_dashboard(\"0.0.0.0:3000\").await?;");
    info!("  This runs the server indefinitely\n");

    // Option 2: Custom setup with state
    info!("Option 2: Custom setup with state");
    
    let state = ApiState::new();
    let dashboard_routes = routes(state.clone());

    // Add custom routes
    let custom_routes = warp::path!("custom" / "data")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "message": "Custom endpoint",
                "timestamp": chrono::Utc::now().timestamp(),
            }))
        });

    let combined_routes = dashboard_routes.or(custom_routes);

    // Start server in background
    let addr = "0.0.0.0:3000";
    info!("Starting dashboard server on http://{}", addr);
    info!("");
    info!("Available endpoints:");
    info!("  GET  /api/health          - Health check");
    info!("  GET  /api/metrics         - System metrics");
    info!("  GET  /api/agents          - List all agents");
    info!("  GET  /api/agents/:id      - Get agent details");
    info!("  PUT  /api/agents/:id      - Update agent");
    info!("  GET  /api/tasks           - List all tasks");
    info!("  POST /api/tasks           - Create new task");
    info!("  GET  /api/tasks/:id       - Get task details");
    info!("  POST /api/tasks/:id/assign - Assign task to agent");
    info!("  POST /api/tasks/:id/cancel - Cancel task");
    info!("  GET  /api/workflows       - List workflows");
    info!("  POST /api/workflows/start  - Start workflow");
    info!("  GET  /api/workflows/:id   - Get workflow details");
    info!("  POST /api/workflows/:id/pause - Pause workflow");
    info!("  POST /api/workflows/:id/resume - Resume workflow");
    info!("  POST /api/workflows/:id/cancel - Cancel workflow");
    info!("  WS   /ws                  - WebSocket for real-time updates");
    info!("  GET  /custom/data         - Custom endpoint example");
    info!("");
    info!("Press Ctrl+C to stop the server");

    // Run server
    warp::serve(combined_routes).run(addr.parse()?).await;

    Ok(())
}

// Alternative: Run with Prometheus metrics endpoint
#[allow(dead_code)]
async fn start_with_prometheus() -> Result<(), Box<dyn std::error::Error>> {
    use dashboard::MetricsCollector;
    use std::sync::Arc;

    let metrics = Arc::new(MetricsCollector::new());
    let metrics_clone = metrics.clone();

    // Metrics endpoint
    let metrics_route = warp::path!("metrics")
        .and(warp::get())
        .map(move || {
            let metrics = metrics_clone.gather().unwrap();
            warp::reply::with_header(
                metrics,
                "Content-Type",
                prometheus::TEXT_FORMAT,
            )
        });

    // Dashboard routes
    let state = ApiState::new();
    let dashboard_routes = routes(state);

    let combined = dashboard_routes.or(metrics_route);

    info!("Starting on http://0.0.0.0:3000");
    info!("Prometheus metrics at http://0.0.0.0:3000/metrics");

    warp::serve(combined)
        .run("0.0.0.0:3000".parse()?)
        .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::test::request;

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = ApiState::new();
        let filter = routes(state);

        let response = request()
            .path("/api/health")
            .method("GET")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_create_task() {
        let state = ApiState::new();
        let filter = routes(state);

        let body = r#"{"description": "Test task"}"#;

        let response = request()
            .path("/api/tasks")
            .method("POST")
            .body(body)
            .header("content-type", "application/json")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
    }
}
