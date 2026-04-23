//! Benchmarking module.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Benchmark configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub name: String,
    pub iterations: usize,
    #[serde(skip)]
    pub operation: Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>>> + Send> + Send + Sync>,
}

impl BenchmarkConfig {
    pub fn new<F, Fut>(name: &str, iterations: usize, operation: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        Self {
            name: name.to_string(),
            iterations,
            operation: Box::new(move || Box::pin(operation())),
        }
    }
}

/// Benchmark result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_name: String,
    pub iterations: usize,
    pub duration_secs: f64,
    pub ops_per_second: f64,
    pub avg_time_per_op_ms: f64,
}

/// Common benchmarks.
pub mod benchmarks {
    use super::*;

    /// Task planning benchmark.
    pub async fn task_planning_benchmark() -> Result<()> {
        // Simulate task planning
        tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
        Ok(())
    }

    /// State sync benchmark.
    pub async fn state_sync_benchmark() -> Result<()> {
        // Simulate state synchronization
        tokio::time::sleep(tokio::time::Duration::from_micros(50)).await;
        Ok(())
    }

    /// Mesh communication benchmark.
    pub async fn mesh_comm_benchmark() -> Result<()> {
        // Simulate mesh communication
        tokio::time::sleep(tokio::time::Duration::from_micros(200)).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark() {
        let config = BenchmarkConfig::new(
            "test_bench",
            100,
            || async { Ok(()) },
        );

        assert_eq!(config.name, "test_bench");
        assert_eq!(config.iterations, 100);
    }

    #[tokio::test]
    async fn test_common_benchmarks() {
        benchmarks::task_planning_benchmark().await.unwrap();
        benchmarks::state_sync_benchmark().await.unwrap();
        benchmarks::mesh_comm_benchmark().await.unwrap();
    }
}
