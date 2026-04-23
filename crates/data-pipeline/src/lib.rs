//! Data pipeline and ETL/ELT processing.
//!
//! Provides:
//! - Extract from multiple sources
//! - Transform with custom logic
//! - Load to various destinations
//! - Pipeline orchestration
//! - Data quality checks
//! - Schema evolution

pub mod extract;
pub mod transform;
pub mod load;
pub mod pipeline;
pub mod quality;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use extract::*;
pub use transform::*;
pub use load::*;
pub use pipeline::*;
pub use quality::*;

/// Data pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub pipeline_type: PipelineType,
    pub batch_size: usize,
    pub parallelism: usize,
    pub enable_checkpointing: bool,
    pub checkpoint_interval_secs: u64,
    pub max_retries: u32,
    pub enable_quality_checks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineType {
    ETL,      // Extract -> Transform -> Load
    ELT,      // Extract -> Load -> Transform
    Streaming, // Real-time streaming
    Batch,    // Batch processing
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            pipeline_type: PipelineType::ETL,
            batch_size: 1000,
            parallelism: 4,
            enable_checkpointing: true,
            checkpoint_interval_secs: 300,
            max_retries: 3,
            enable_quality_checks: true,
        }
    }
}

/// Pipeline manager.
pub struct PipelineManager {
    config: PipelineConfig,
    pipelines: RwLock<HashMap<String, DataPipeline>>,
    execution_history: RwLock<Vec<ExecutionRecord>>,
}

impl PipelineManager {
    /// Create new pipeline manager.
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            pipelines: RwLock::new(HashMap::new()),
            execution_history: RwLock::new(Vec::new()),
        }
    }

    /// Initialize pipeline manager.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing data pipeline manager");
        Ok(())
    }

    /// Register pipeline.
    pub async fn register_pipeline(&self, pipeline: DataPipeline) -> Result<()> {
        let mut pipelines = self.pipelines.write().await;
        let id = pipeline.id.clone();
        pipelines.insert(id.clone(), pipeline);
        info!("Pipeline registered: {}", id);
        Ok(())
    }

    /// Execute pipeline.
    pub async fn execute_pipeline(&self, pipeline_id: &str) -> Result<ExecutionResult> {
        let pipelines = self.pipelines.read().await;
        
        let pipeline = pipelines.get(pipeline_id)
            .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", pipeline_id))?;

        let start_time = chrono::Utc::now();
        let result = pipeline.execute(&self.config).await?;
        let end_time = chrono::Utc::now();

        // Record execution
        let record = ExecutionRecord {
            pipeline_id: pipeline_id.to_string(),
            start_time,
            end_time,
            status: if result.success { "success" } else { "failed" }.to_string(),
            records_processed: result.records_processed,
            error_message: result.error_message.clone(),
        };

        self.execution_history.write().await.push(record);

        Ok(result)
    }

    /// Schedule pipeline.
    pub async fn schedule_pipeline(&self, pipeline_id: &str, cron_expression: &str) -> Result<String> {
        // Would integrate with scheduler
        let schedule_id = uuid::Uuid::new_v4().to_string();
        info!("Pipeline {} scheduled: {}", pipeline_id, cron_expression);
        Ok(schedule_id)
    }

    /// Get pipeline status.
    pub async fn get_pipeline_status(&self, pipeline_id: &str) -> Result<PipelineStatus> {
        let pipelines = self.pipelines.read().await;
        
        let pipeline = pipelines.get(pipeline_id)
            .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", pipeline_id))?;

        Ok(PipelineStatus {
            id: pipeline.id.clone(),
            name: pipeline.name.clone(),
            enabled: pipeline.enabled,
            last_run: pipeline.last_execution.clone(),
            next_run: None,
            status: "idle".to_string(),
        })
    }

    /// Get execution history.
    pub async fn get_execution_history(&self, pipeline_id: Option<&str>, limit: usize) -> Vec<ExecutionRecord> {
        let history = self.execution_history.read().await;
        
        if let Some(pid) = pipeline_id {
            history.iter()
                .filter(|r| r.pipeline_id == pid)
                .rev()
                .take(limit)
                .cloned()
                .collect()
        } else {
            history.iter()
                .rev()
                .take(limit)
                .cloned()
                .collect()
        }
    }

    /// Get pipeline statistics.
    pub async fn get_stats(&self) -> Result<PipelineStats> {
        let pipelines = self.pipelines.read().await;
        let history = self.execution_history.read().await;

        let total_pipelines = pipelines.len();
        let total_executions = history.len();
        let successful_executions = history.iter().filter(|r| r.status == "success").count();
        
        let total_records: i64 = history.iter().map(|r| r.records_processed).sum();

        Ok(PipelineStats {
            total_pipelines: total_pipelines as i32,
            total_executions: total_executions as i32,
            successful_executions: successful_executions as i32,
            failed_executions: (total_executions - successful_executions) as i32,
            total_records_processed: total_records,
            success_rate: if total_executions > 0 {
                successful_executions as f64 / total_executions as f64
            } else {
                0.0
            },
        })
    }
}

/// Data pipeline definition.
#[derive(Debug, Clone)]
pub struct DataPipeline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pipeline_type: PipelineType,
    pub extractors: Vec<Extractor>,
    pub transformers: Vec<Transform>,
    pub loaders: Vec<Loader>,
    pub quality_checks: Vec<QualityCheck>,
    pub enabled: bool,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

impl DataPipeline {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            pipeline_type: PipelineType::ETL,
            extractors: Vec::new(),
            transformers: Vec::new(),
            loaders: Vec::new(),
            quality_checks: Vec::new(),
            enabled: true,
            last_execution: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_type(mut self, pipeline_type: PipelineType) -> Self {
        self.pipeline_type = pipeline_type;
        self
    }

    pub fn with_extractor(mut self, extractor: Extractor) -> Self {
        self.extractors.push(extractor);
        self
    }

    pub fn with_transformer(mut self, transform: Transform) -> Self {
        self.transformers.push(transform);
        self
    }

    pub fn with_loader(mut self, loader: Loader) -> Self {
        self.loaders.push(loader);
        self
    }

    pub fn with_quality_check(mut self, check: QualityCheck) -> Self {
        self.quality_checks.push(check);
        self
    }

    pub async fn execute(&self, config: &PipelineConfig) -> Result<ExecutionResult> {
        let start_time = chrono::Utc::now();
        let mut records_processed = 0i64;
        let mut error_message = None;

        // Execute based on pipeline type
        match self.pipeline_type {
            PipelineType::ETL => {
                // Extract
                let mut data = Vec::new();
                for extractor in &self.extractors {
                    let extracted = extractor.extract().await?;
                    data.extend(extracted);
                }

                // Transform
                let mut transformed = data;
                for transform in &self.transformers {
                    transformed = transform.apply(transformed).await?;
                }

                // Quality checks
                if config.enable_quality_checks {
                    for check in &self.quality_checks {
                        if !check.validate(&transformed).await? {
                            return Ok(ExecutionResult {
                                success: false,
                                records_processed: 0,
                                error_message: Some("Quality check failed".to_string()),
                                execution_time_ms: 0.0,
                            });
                        }
                    }
                }

                // Load
                records_processed = transformed.len() as i64;
                for loader in &self.loaders {
                    loader.load(transformed.clone()).await?;
                }
            }
            PipelineType::ELT => {
                // Extract
                let mut data = Vec::new();
                for extractor in &self.extractors {
                    let extracted = extractor.extract().await?;
                    data.extend(extracted);
                }

                // Load first
                records_processed = data.len() as i64;
                for loader in &self.loaders {
                    loader.load(data.clone()).await?;
                }

                // Transform in destination
                for transform in &self.transformers {
                    transform.apply_in_place().await?;
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Pipeline type not implemented"));
            }
        }

        let end_time = chrono::Utc::now();
        let execution_time = (end_time - start_time).num_milliseconds() as f64;

        Ok(ExecutionResult {
            success: true,
            records_processed,
            error_message: None,
            execution_time_ms: execution_time,
        })
    }
}

/// Execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub records_processed: i64,
    pub error_message: Option<String>,
    pub execution_time_ms: f64,
}

/// Execution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub pipeline_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub records_processed: i64,
    pub error_message: Option<String>,
}

/// Pipeline status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    pub next_run: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
}

/// Pipeline statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_pipelines: i32,
    pub total_executions: i32,
    pub successful_executions: i32,
    pub failed_executions: i32,
    pub total_records_processed: i64,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_manager() {
        let config = PipelineConfig::default();
        let manager = PipelineManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Create pipeline
        let pipeline = DataPipeline::new("test-pipeline", "Test Pipeline")
            .with_description("Test ETL pipeline");

        manager.register_pipeline(pipeline).await.unwrap();

        // Get status
        let status = manager.get_pipeline_status("test-pipeline").await.unwrap();
        assert_eq!(status.name, "Test Pipeline");

        // Get stats
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_pipelines, 1);
    }
}
