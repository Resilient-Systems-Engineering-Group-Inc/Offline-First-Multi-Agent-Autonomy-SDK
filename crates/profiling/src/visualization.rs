//! Performance visualization tools for distributed systems.
//!
//! This module provides utilities to visualize performance metrics,
//! generate charts, and create human‑readable reports.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use crate::performance_analysis::{LatencyStats, PerformanceAnalyzer};
use crate::distributed_analysis::{Bottleneck, MetricCorrelation, PerformanceAnomaly};

/// Chart type for visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartType {
    /// Line chart (time series).
    Line,
    /// Bar chart (categorical).
    Bar,
    /// Scatter plot (correlation).
    Scatter,
    /// Heat map (two‑dimensional density).
    Heatmap,
    /// Histogram (distribution).
    Histogram,
}

/// Data point for a chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// X‑axis value (often timestamp or category).
    pub x: f64,
    /// Y‑axis value (metric value).
    pub y: f64,
    /// Optional label.
    pub label: Option<String>,
    /// Optional metadata.
    pub metadata: HashMap<String, String>,
}

/// A complete chart definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    /// Chart title.
    pub title: String,
    /// Chart type.
    pub chart_type: ChartType,
    /// X‑axis label.
    pub x_label: String,
    /// Y‑axis label.
    pub y_label: String,
    /// Data series (each series is a line/bar set).
    pub series: Vec<DataSeries>,
    /// Optional configuration (colors, grid, etc.).
    pub config: ChartConfig,
}

/// A series of data points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    /// Series name.
    pub name: String,
    /// Data points.
    pub points: Vec<DataPoint>,
    /// Color (hex, e.g., "#FF5733").
    pub color: Option<String>,
}

/// Chart configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Show grid.
    pub show_grid: bool,
    /// Show legend.
    pub show_legend: bool,
    /// Time format for x‑axis (if time series).
    pub time_format: Option<String>,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            show_grid: true,
            show_legend: true,
            time_format: Some("%H:%M:%S".to_string()),
        }
    }
}

/// Performance report with visualizations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report title.
    pub title: String,
    /// Generation timestamp.
    pub generated_at: SystemTime,
    /// Time range covered.
    pub time_range: (SystemTime, SystemTime),
    /// Charts included in the report.
    pub charts: Vec<Chart>,
    /// Summary statistics.
    pub summary: HashMap<String, f64>,
    /// Detected bottlenecks.
    pub bottlenecks: Vec<Bottleneck>,
    /// Detected anomalies.
    pub anomalies: Vec<PerformanceAnomaly>,
    /// Metric correlations.
    pub correlations: Vec<MetricCorrelation>,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

/// Chart generator that creates visualizations from performance data.
pub struct ChartGenerator {
    /// Default chart configuration.
    default_config: ChartConfig,
}

impl ChartGenerator {
    /// Create a new chart generator.
    pub fn new() -> Self {
        Self {
            default_config: ChartConfig::default(),
        }
    }

    /// Generate a latency trend chart from latency stats over time.
    pub fn latency_trend_chart(
        &self,
        latency_stats_over_time: &[(SystemTime, LatencyStats)],
        title: &str,
    ) -> Chart {
        let mut series = DataSeries {
            name: "Latency (ms)".to_string(),
            points: Vec::new(),
            color: Some("#3366FF".to_string()),
        };

        for (timestamp, stats) in latency_stats_over_time {
            let x = timestamp.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let y = stats.mean.as_millis() as f64;
            series.points.push(DataPoint {
                x,
                y,
                label: Some(format!("Mean: {:.2} ms", y)),
                metadata: HashMap::from([
                    ("p50".to_string(), stats.p50.as_millis().to_string()),
                    ("p95".to_string(), stats.p95.as_millis().to_string()),
                    ("p99".to_string(), stats.p99.as_millis().to_string()),
                ]),
            });
        }

        Chart {
            title: title.to_string(),
            chart_type: ChartType::Line,
            x_label: "Time".to_string(),
            y_label: "Latency (ms)".to_string(),
            series: vec![series],
            config: self.default_config.clone(),
        }
    }

    /// Generate a throughput chart.
    pub fn throughput_chart(
        &self,
        throughput_samples: &[(SystemTime, f64)],
        title: &str,
    ) -> Chart {
        let mut series = DataSeries {
            name: "Throughput (ops/sec)".to_string(),
            points: Vec::new(),
            color: Some("#33FF66".to_string()),
        };

        for (timestamp, throughput) in throughput_samples {
            let x = timestamp.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            series.points.push(DataPoint {
                x,
                y: *throughput,
                label: Some(format!("{:.1} ops/sec", throughput)),
                metadata: HashMap::new(),
            });
        }

        Chart {
            title: title.to_string(),
            chart_type: ChartType::Line,
            x_label: "Time".to_string(),
            y_label: "Throughput (ops/sec)".to_string(),
            series: vec![series],
            config: self.default_config.clone(),
        }
    }

    /// Generate a correlation heatmap between multiple metrics.
    pub fn correlation_heatmap(
        &self,
        correlations: &[MetricCorrelation],
        title: &str,
    ) -> Chart {
        // Group correlations by metric pairs
        let mut series = DataSeries {
            name: "Correlation".to_string(),
            points: Vec::new(),
            color: None, // Heatmap uses color scale
        };

        // For simplicity, we'll create a scatter plot of correlation values
        // In a real implementation, this would be a proper heatmap
        for (i, corr) in correlations.iter().enumerate() {
            series.points.push(DataPoint {
                x: i as f64,
                y: corr.correlation,
                label: Some(format!("{} vs {}", corr.metric_a, corr.metric_b)),
                metadata: HashMap::from([
                    ("metric_a".to_string(), corr.metric_a.clone()),
                    ("metric_b".to_string(), corr.metric_b.clone()),
                    ("sample_count".to_string(), corr.sample_count.to_string()),
                    ("significant".to_string(), corr.significant.to_string()),
                ]),
            });
        }

        Chart {
            title: title.to_string(),
            chart_type: ChartType::Scatter,
            x_label: "Metric Pair".to_string(),
            y_label: "Correlation Coefficient".to_string(),
            series: vec![series],
            config: self.default_config.clone(),
        }
    }

    /// Generate a bottleneck severity chart.
    pub fn bottleneck_chart(
        &self,
        bottlenecks: &[Bottleneck],
        title: &str,
    ) -> Chart {
        let mut series = DataSeries {
            name: "Bottleneck Severity".to_string(),
            points: Vec::new(),
            color: Some("#FF3333".to_string()),
        };

        for (i, bottleneck) in bottlenecks.iter().enumerate() {
            series.points.push(DataPoint {
                x: i as f64,
                y: bottleneck.severity * 100.0, // as percentage
                label: Some(bottleneck.component.clone()),
                metadata: HashMap::from([
                    ("metric".to_string(), bottleneck.metric.clone()),
                    ("current_value".to_string(), bottleneck.current_value.to_string()),
                    ("threshold".to_string(), bottleneck.threshold.to_string()),
                ]),
            });
        }

        Chart {
            title: title.to_string(),
            chart_type: ChartType::Bar,
            x_label: "Bottleneck".to_string(),
            y_label: "Severity (%)".to_string(),
            series: vec![series],
            config: self.default_config.clone(),
        }
    }
}

/// Report generator that creates comprehensive performance reports.
pub struct ReportGenerator {
    chart_generator: ChartGenerator,
}

impl ReportGenerator {
    /// Create a new report generator.
    pub fn new() -> Self {
        Self {
            chart_generator: ChartGenerator::new(),
        }
    }

    /// Generate a comprehensive performance report.
    pub async fn generate_report(
        &self,
        analyzer: &PerformanceAnalyzer,
        start_time: SystemTime,
        end_time: SystemTime,
        title: &str,
    ) -> PerformanceReport {
        // Collect data from analyzer (simplified - in reality would query metrics)
        let latency_stats = analyzer.latency_stats("overall").await;
        // For demonstration, create dummy data
        let latency_stats_over_time = vec![
            (start_time, LatencyStats {
                mean: Duration::from_millis(50),
                p50: Duration::from_millis(45),
                p95: Duration::from_millis(90),
                p99: Duration::from_millis(120),
                min: Duration::from_millis(10),
                max: Duration::from_millis(150),
                sample_count: 1000,
            }),
            (end_time, LatencyStats {
                mean: Duration::from_millis(55),
                p50: Duration::from_millis(50),
                p95: Duration::from_millis(95),
                p99: Duration::from_millis(125),
                min: Duration::from_millis(12),
                max: Duration::from_millis(160),
                sample_count: 1200,
            }),
        ];

        let throughput_samples = vec![
            (start_time, 1000.0),
            (end_time, 950.0),
        ];

        // Generate charts
        let latency_chart = self.chart_generator
            .latency_trend_chart(&latency_stats_over_time, "Latency Trend");
        let throughput_chart = self.chart_generator
            .throughput_chart(&throughput_samples, "Throughput Trend");

        // Create summary statistics
        let mut summary = HashMap::new();
        if let Some(stats) = latency_stats {
            summary.insert("mean_latency_ms".to_string(), stats.mean.as_millis() as f64);
            summary.insert("p95_latency_ms".to_string(), stats.p95.as_millis() as f64);
            summary.insert("sample_count".to_string(), stats.sample_count as f64);
        }

        // Get bottlenecks and anomalies (would come from distributed analyzer)
        let bottlenecks = Vec::new(); // In real implementation, query bottleneck detector
        let anomalies = Vec::new();
        let correlations = Vec::new();

        PerformanceReport {
            title: title.to_string(),
            generated_at: SystemTime::now(),
            time_range: (start_time, end_time),
            charts: vec![latency_chart, throughput_chart],
            summary,
            bottlenecks,
            anomalies,
            correlations,
            recommendations: vec![
                "Consider increasing mesh‑transport queue size".to_string(),
                "Monitor CPU usage on agent 3".to_string(),
            ],
        }
    }

    /// Export report to JSON format.
    pub fn export_json(&self, report: &PerformanceReport) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(report)
    }

    /// Export report to HTML format (simplified).
    pub fn export_html(&self, report: &PerformanceReport) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Performance Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: sans-serif; margin: 20px; }\n");
        html.push_str(".chart { border: 1px solid #ccc; padding: 10px; margin: 10px 0; }\n");
        html.push_str(".summary { background: #f5f5f5; padding: 10px; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        html.push_str(&format!("<h1>{}</h1>\n", report.title));
        html.push_str(&format!("<p>Generated at: {:?}</p>\n", report.generated_at));
        
        // Summary
        html.push_str("<div class=\"summary\">\n<h2>Summary</h2>\n<ul>\n");
        for (key, value) in &report.summary {
            html.push_str(&format!("<li>{}: {:.2}</li>\n", key, value));
        }
        html.push_str("</ul>\n</div>\n");
        
        // Charts placeholder
        html.push_str("<h2>Charts</h2>\n");
        for chart in &report.charts {
            html.push_str(&format!("<div class=\"chart\">\n<h3>{}</h3>\n", chart.title));
            html.push_str(&format!("<p>Type: {:?}, X: {}, Y: {}</p>\n", 
                chart.chart_type, chart.x_label, chart.y_label));
            html.push_str("<p>[Chart visualization would appear here]</p>\n");
            html.push_str("</div>\n");
        }
        
        // Recommendations
        if !report.recommendations.is_empty() {
            html.push_str("<h2>Recommendations</h2>\n<ul>\n");
            for rec in &report.recommendations {
                html.push_str(&format!("<li>{}</li>\n", rec));
            }
            html.push_str("</ul>\n");
        }
        
        html.push_str("</body>\n</html>");
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_chart_generator() {
        let generator = ChartGenerator::new();
        
        let now = SystemTime::now();
        let stats = vec![
            (now, LatencyStats {
                mean: Duration::from_millis(50),
                p50: Duration::from_millis(45),
                p95: Duration::from_millis(90),
                p99: Duration::from_millis(120),
                min: Duration::from_millis(10),
                max: Duration::from_millis(150),
                sample_count: 1000,
            }),
        ];
        
        let chart = generator.latency_trend_chart(&stats, "Test Chart");
        assert_eq!(chart.title, "Test Chart");
        assert_eq!(chart.chart_type, ChartType::Line);
        assert_eq!(chart.series.len(), 1);
    }

    #[test]
    fn test_report_generator() {
        let generator = ReportGenerator::new();
        let report = PerformanceReport {
            title: "Test Report".to_string(),
            generated_at: SystemTime::now(),
            time_range: (SystemTime::now(), SystemTime::now()),
            charts: Vec::new(),
            summary: HashMap::new(),
            bottlenecks: Vec::new(),
            anomalies: Vec::new(),
            correlations: Vec::new(),
            recommendations: vec!["Test recommendation".to_string()],
        };
        
        let json = generator.export_json(&report);
        assert!(json.is_ok());
        
        let html = generator.export_html(&report);
        assert!(html.contains("Test Report"));
        assert!(html.contains("Test recommendation"));
    }
}