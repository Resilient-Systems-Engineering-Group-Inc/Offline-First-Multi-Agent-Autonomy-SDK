//! Performance testing framework for the Multi-Agent SDK.
//!
//! Provides:
//! - Load testing
//! - Stress testing
//! - Benchmarking
//! - Performance metrics collection
//! - Automated performance regression detection

pub mod benchmark;
pub mod load_test;
pub mod stress_test;
pub mod metrics;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use benchmark::*;
pub use load_test::*;
pub use stress_test::*;
pub use metrics::*;

/// Performance test configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfTestConfig {
    pub target_url: String,
    pub concurrent_users: usize,
    pub test_duration_secs: u64,
    pub ramp_up_secs: u64,
    pub metrics_endpoint: Option<String>,
    pub alert_thresholds: HashMap<String, f64>,
}

impl Default for PerfTestConfig {
    fn default() -> Self {
        let mut alert_thresholds = HashMap::new();
        alert_thresholds.insert("response_time_p95_ms".to_string(), 100.0);
        alert_thresholds.insert("error_rate_percent".to_string(), 1.0);
        alert_thresholds.insert("requests_per_second".to_string(), 1000.0);

        Self {
            target_url: "http://localhost:8000".to_string(),
            concurrent_users: 100,
            test_duration_secs: 60,
            ramp_up_secs: 10,
            metrics_endpoint: None,
            alert_thresholds,
        }
    }
}

/// Performance test manager.
pub struct PerfTestManager {
    config: PerfTestConfig,
    results: RwLock<Vec<TestResult>>,
    running_tests: RwLock<HashMap<String, bool>>,
}

impl PerfTestManager {
    /// Create new performance test manager.
    pub fn new(config: PerfTestConfig) -> Self {
        Self {
            config,
            results: RwLock::new(vec![]),
            running_tests: RwLock::new(HashMap::new()),
        }
    }

    /// Run load test.
    pub async fn run_load_test(&self, name: &str, config: LoadTestConfig) -> Result<TestResult> {
        info!("Starting load test: {}", name);

        self.running_tests.write().await.insert(name.to_string(), true);

        let result = run_load_test_impl(&self.config, &config).await?;

        self.running_tests.write().await.remove(name);
        self.results.write().await.push(result.clone());

        info!("Load test completed: {} - {} requests/sec", name, result.requests_per_second);
        Ok(result)
    }

    /// Run stress test.
    pub async fn run_stress_test(&self, name: &str, config: StressTestConfig) -> Result<TestResult> {
        info!("Starting stress test: {}", name);

        self.running_tests.write().await.insert(name.to_string(), true);

        let result = run_stress_test_impl(&self.config, &config).await?;

        self.running_tests.write().await.remove(name);
        self.results.write().await.push(result.clone());

        info!("Stress test completed: {} - {} requests/sec", name, result.requests_per_second);
        Ok(result)
    }

    /// Run benchmark.
    pub async fn run_benchmark(&self, name: &str, config: BenchmarkConfig) -> Result<BenchmarkResult> {
        info!("Running benchmark: {}", name);

        let result = run_benchmark_impl(&config).await?;

        info!("Benchmark completed: {} - {} ops/sec", name, result.ops_per_second);
        Ok(result)
    }

    /// Get all test results.
    pub async fn get_results(&self) -> Vec<TestResult> {
        let results = self.results.read().await;
        results.clone()
    }

    /// Get last test result.
    pub async fn get_last_result(&self) -> Option<TestResult> {
        let results = self.results.read().await;
        results.last().cloned()
    }

    /// Check if test is running.
    pub async fn is_test_running(&self, name: &str) -> bool {
        let running = self.running_tests.read().await;
        running.get(name).copied().unwrap_or(false)
    }

    /// Get performance report.
    pub async fn get_report(&self) -> PerformanceReport {
        let results = self.results.read().await;

        let total_tests = results.len();
        let avg_response_time: f64 = results.iter()
            .map(|r| r.avg_response_time_ms)
            .sum::<f64>() / total_tests.max(1) as f64;

        let avg_requests_per_second: f64 = results.iter()
            .map(|r| r.requests_per_second)
            .sum::<f64>() / total_tests.max(1) as f64;

        let error_rate: f64 = results.iter()
            .map(|r| r.error_rate_percent)
            .sum::<f64>() / total_tests.max(1) as f64;

        PerformanceReport {
            total_tests,
            avg_response_time_ms: avg_response_time,
            avg_requests_per_second,
            avg_error_rate_percent: error_rate,
            latest_results: results.iter().take(5).cloned().collect(),
        }
    }

    /// Compare with baseline.
    pub async fn compare_with_baseline(&self, baseline: &PerformanceReport) -> PerformanceComparison {
        let current = self.get_report().await;

        let response_time_change = ((current.avg_response_time_ms - baseline.avg_response_time_ms) 
            / baseline.avg_response_time_ms.max(0.01)) * 100.0;

        let throughput_change = ((current.avg_requests_per_second - baseline.avg_requests_per_second) 
            / baseline.avg_requests_per_second.max(0.01)) * 100.0;

        let error_rate_change = current.avg_error_rate_percent - baseline.avg_error_rate_percent;

        PerformanceComparison {
            response_time_change_percent: response_time_change,
            throughput_change_percent: throughput_change,
            error_rate_change_percent: error_rate_change,
            regression_detected: response_time_change > 10.0 || error_rate_change > 1.0,
        }
    }
}

/// Test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub test_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_secs: f64,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub requests_per_second: f64,
    pub avg_response_time_ms: f64,
    pub p50_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub min_response_time_ms: f64,
    pub max_response_time_ms: f64,
}

/// Performance report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_tests: usize,
    pub avg_response_time_ms: f64,
    pub avg_requests_per_second: f64,
    pub avg_error_rate_percent: f64,
    pub latest_results: Vec<TestResult>,
}

/// Performance comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub response_time_change_percent: f64,
    pub throughput_change_percent: f64,
    pub error_rate_change_percent: f64,
    pub regression_detected: bool,
}

/// Run load test implementation.
async fn run_load_test_impl(config: &PerfTestConfig, load_config: &LoadTestConfig) -> Result<TestResult> {
    let start = std::time::Instant::now();
    let mut requests = vec![];

    // Simulate concurrent users
    let mut handles = vec![];
    for i in 0..config.concurrent_users {
        let url = config.target_url.clone();
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::new();
            let mut results = vec![];

            for _ in 0..(load_config.requests_per_user / config.concurrent_users as u64) {
                let req_start = std::time::Instant::now();
                
                let response = client.get(&url).send().await;
                
                let req_duration = req_start.elapsed().as_secs_f64() * 1000.0;
                
                match response {
                    Ok(resp) => {
                        results.push((req_duration, resp.status().is_success()));
                    }
                    Err(_) => {
                        results.push((req_duration, false));
                    }
                }
            }

            (i, results)
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let (_, user_results) = handle.await?;
        requests.extend(user_results);
    }

    let duration = start.elapsed().as_secs_f64();
    let total_requests = requests.len() as i64;
    let successful = requests.iter().filter(|(_, success)| *success).count() as i64;
    let failed = total_requests - successful;

    let response_times: Vec<f64> = requests.iter().map(|(time, _)| *time).collect();
    response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg_response_time = response_times.iter().sum::<f64>() / response_times.len() as f64;
    let p50_idx = (response_times.len() as f64 * 0.5) as usize;
    let p95_idx = (response_times.len() as f64 * 0.95) as usize;
    let p99_idx = (response_times.len() as f64 * 0.99) as usize;

    Ok(TestResult {
        test_name: load_config.name.clone(),
        test_type: "load".to_string(),
        timestamp: chrono::Utc::now(),
        duration_secs: duration,
        total_requests,
        successful_requests: successful,
        failed_requests: failed,
        requests_per_second: total_requests as f64 / duration,
        avg_response_time_ms: avg_response_time,
        p50_response_time_ms: response_times.get(p50_idx).copied().unwrap_or(0.0),
        p95_response_time_ms: response_times.get(p95_idx).copied().unwrap_or(0.0),
        p99_response_time_ms: response_times.get(p99_idx).copied().unwrap_or(0.0),
        error_rate_percent: (failed as f64 / total_requests as f64) * 100.0,
        min_response_time_ms: response_times.first().copied().unwrap_or(0.0),
        max_response_time_ms: response_times.last().copied().unwrap_or(0.0),
    })
}

/// Run stress test implementation.
async fn run_stress_test_impl(config: &PerfTestConfig, stress_config: &StressTestConfig) -> Result<TestResult> {
    // Similar to load test but with increasing load
    run_load_test_impl(config, &LoadTestConfig {
        name: stress_config.name.clone(),
        requests_per_user: stress_config.max_requests_per_user,
    }).await
}

/// Run benchmark implementation.
async fn run_benchmark_impl(config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let iterations = config.iterations;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        config.operation().await?;
    }

    let duration = start.elapsed().as_secs_f64();

    Ok(BenchmarkResult {
        benchmark_name: config.name.clone(),
        iterations,
        duration_secs: duration,
        ops_per_second: iterations as f64 / duration,
        avg_time_per_op_ms: (duration / iterations as f64) * 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_perf_test_manager() {
        let config = PerfTestConfig::default();
        let manager = PerfTestManager::new(config);

        // Run benchmark
        let bench_config = BenchmarkConfig {
            name: "test_benchmark".to_string(),
            iterations: 100,
            operation: Box::new(|| async { Ok(()) }),
        };

        let result = manager.run_benchmark("test", &bench_config).await.unwrap();
        assert!(result.ops_per_second > 0.0);
    }
}
