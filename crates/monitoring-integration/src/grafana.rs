//! Grafana dashboard integration.

use crate::error::{MonitoringError, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Grafana API client.
pub struct GrafanaClient {
    base_url: String,
    api_key: String,
    client: Client,
}

/// Dashboard model (simplified).
#[derive(Debug, Serialize, Deserialize)]
pub struct Dashboard {
    /// Dashboard title.
    pub title: String,
    /// Dashboard tags.
    pub tags: Vec<String>,
    /// Panels (simplified).
    pub panels: Vec<Panel>,
    /// Time range.
    pub time: TimeRange,
    /// Refresh interval.
    pub refresh: String,
}

/// Panel model.
#[derive(Debug, Serialize, Deserialize)]
pub struct Panel {
    /// Panel title.
    pub title: String,
    /// Panel type (graph, singlestat, table, etc.)
    #[serde(rename = "type")]
    pub panel_type: String,
    /// Datasource.
    pub datasource: String,
    /// Targets (queries).
    pub targets: Vec<Target>,
    /// Grid position.
    pub gridPos: GridPos,
}

/// Target (query) model.
#[derive(Debug, Serialize, Deserialize)]
pub struct Target {
    /// Prometheus query expression.
    pub expr: String,
    /// Legend format.
    pub legendFormat: String,
    /// Ref ID.
    pub refId: String,
}

/// Grid position.
#[derive(Debug, Serialize, Deserialize)]
pub struct GridPos {
    /// X coordinate.
    pub x: u32,
    /// Y coordinate.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// Time range.
#[derive(Debug, Serialize, Deserialize)]
pub struct TimeRange {
    /// From time.
    pub from: String,
    /// To time.
    pub to: String,
}

/// Response from Grafana API when creating/updating a dashboard.
#[derive(Debug, Deserialize)]
pub struct DashboardResponse {
    /// Dashboard ID.
    pub id: Option<u64>,
    /// Dashboard slug.
    pub slug: String,
    /// Dashboard status.
    pub status: String,
    /// Dashboard version.
    pub version: u64,
    /// Dashboard URL.
    pub url: String,
}

impl GrafanaClient {
    /// Create a new Grafana client.
    pub fn new(base_url: String, api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        Self {
            base_url,
            api_key,
            client,
        }
    }

    /// Create or update a dashboard.
    pub async fn create_dashboard(&self, dashboard: &Dashboard) -> Result<DashboardResponse> {
        let url = format!("{}/api/dashboards/db", self.base_url);
        let payload = serde_json::json!({
            "dashboard": dashboard,
            "overwrite": true,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(MonitoringError::HttpClient)?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let resp: DashboardResponse = response
                    .json()
                    .await
                    .map_err(MonitoringError::HttpClient)?;
                log::info!("Dashboard created/updated: {}", resp.url);
                Ok(resp)
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(MonitoringError::Other(format!(
                    "Grafana API error {}: {}",
                    status, text
                )))
            }
        }
    }

    /// Delete a dashboard by UID.
    pub async fn delete_dashboard(&self, uid: &str) -> Result<()> {
        let url = format!("{}/api/dashboards/uid/{}", self.base_url, uid);
        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(MonitoringError::HttpClient)?;

        if response.status().is_success() {
            log::info!("Dashboard deleted: {}", uid);
            Ok(())
        } else {
            let text = response.text().await.unwrap_or_default();
            Err(MonitoringError::Other(format!(
                "Failed to delete dashboard {}: {}",
                uid, text
            )))
        }
    }

    /// List all dashboards.
    pub async fn list_dashboards(&self) -> Result<Vec<DashboardResponse>> {
        let url = format!("{}/api/search?type=dash-db", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(MonitoringError::HttpClient)?;

        if response.status().is_success() {
            let dashboards: Vec<DashboardResponse> = response
                .json()
                .await
                .map_err(MonitoringError::HttpClient)?;
            Ok(dashboards)
        } else {
            let text = response.text().await.unwrap_or_default();
            Err(MonitoringError::Other(format!(
                "Failed to list dashboards: {}",
                text
            )))
        }
    }
}

/// Predefined dashboard templates.
pub mod templates {
    use super::*;

    /// Create a default dashboard for the offline-first SDK.
    pub fn default_agent_dashboard() -> Dashboard {
        Dashboard {
            title: "Offline-First Multi-Agent Autonomy SDK",
            tags: vec!["offline-first".to_string(), "multi-agent".to_string()],
            panels: vec![
                Panel {
                    title: "Tasks Created",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "rate(offline_first_tasks_created_total[5m])".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 0, y: 0, w: 12, h: 8 },
                },
                Panel {
                    title: "Pending Tasks",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "offline_first_pending_tasks".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 12, y: 0, w: 12, h: 8 },
                },
                Panel {
                    title: "CPU Usage",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "offline_first_cpu_usage_percent".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 0, y: 8, w: 8, h: 8 },
                },
                Panel {
                    title: "Memory Usage",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "offline_first_memory_usage_bytes".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 8, y: 8, w: 8, h: 8 },
                },
                Panel {
                    title: "Network Latency",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "histogram_quantile(0.95, sum(rate(offline_first_network_latency_seconds_bucket[5m])) by (le, instance))".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 16, y: 8, w: 8, h: 8 },
                },
                Panel {
                    title: "Health Status",
                    panel_type: "singlestat".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "offline_first_health_status".to_string(),
                        legendFormat: "".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 0, y: 16, w: 6, h: 4 },
                },
            ],
            time: TimeRange {
                from: "now-1h".to_string(),
                to: "now".to_string(),
            },
            refresh: "10s".to_string(),
        }
    }

    /// Create a dashboard for distributed planning.
    pub fn planning_dashboard() -> Dashboard {
        Dashboard {
            title: "Distributed Planning Metrics",
            tags: vec!["planning".to_string(), "distributed".to_string()],
            panels: vec![
                Panel {
                    title: "Task Assignment Rate",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "rate(offline_first_tasks_assigned_total[5m])".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 0, y: 0, w: 12, h: 8 },
                },
                Panel {
                    title: "Task Completion Rate",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "rate(offline_first_tasks_completed_total[5m])".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 12, y: 0, w: 12, h: 8 },
                },
                Panel {
                    title: "Missed Deadlines",
                    panel_type: "graph".to_string(),
                    datasource: "Prometheus".to_string(),
                    targets: vec![Target {
                        expr: "rate(offline_first_tasks_missed_deadline_total[5m])".to_string(),
                        legendFormat: "{{instance}}".to_string(),
                        refId: "A".to_string(),
                    }],
                    gridPos: GridPos { x: 0, y: 8, w: 12, h: 8 },
                },
            ],
            time: TimeRange {
                from: "now-1h".to_string(),
                to: "now".to_string(),
            },
            refresh: "10s".to_string(),
        }
    }
}