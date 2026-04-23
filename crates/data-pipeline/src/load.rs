//! Data loaders.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Data loader trait.
#[async_trait::async_trait]
pub trait LoaderTrait: Send + Sync {
    async fn load(&self, data: Vec<serde_json::Value>) -> Result<usize>;
    fn destination_type(&self) -> &str;
}

/// Loader.
#[derive(Debug, Clone)]
pub struct Loader {
    pub id: String,
    pub name: String,
    pub destination_type: DestinationType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DestinationType {
    Database,
    DataWarehouse,
    DataLake,
    API,
    File,
    MessageQueue,
    Cache,
    Custom,
}

impl Loader {
    pub fn new(id: &str, name: &str, destination_type: DestinationType) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            destination_type,
            config: serde_json::json!({}),
        }
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    pub async fn load(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        match self.destination_type {
            DestinationType::Database => self.load_database(data).await,
            DestinationType::DataWarehouse => self.load_warehouse(data).await,
            DestinationType::DataLake => self.load_lake(data).await,
            DestinationType::API => self.load_api(data).await,
            DestinationType::File => self.load_file(data).await,
            DestinationType::MessageQueue => self.load_mq(data).await,
            DestinationType::Cache => self.load_cache(data).await,
            DestinationType::Custom => self.load_custom(data).await,
        }
    }

    async fn load_database(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to database: {} records", data.len());
        Ok(data.len())
    }

    async fn load_warehouse(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to data warehouse: {} records", data.len());
        Ok(data.len())
    }

    async fn load_lake(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to data lake: {} records", data.len());
        Ok(data.len())
    }

    async fn load_api(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to API: {} records", data.len());
        Ok(data.len())
    }

    async fn load_file(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to file: {} records", data.len());
        Ok(data.len())
    }

    async fn load_mq(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to message queue: {} records", data.len());
        Ok(data.len())
    }

    async fn load_cache(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading to cache: {} records", data.len());
        Ok(data.len())
    }

    async fn load_custom(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        tracing::info!("Loading custom: {} records", data.len());
        Ok(data.len())
    }
}

/// Database loader.
pub struct DatabaseLoader {
    connection_string: String,
    table: String,
    batch_size: usize,
}

impl DatabaseLoader {
    pub fn new(connection_string: &str, table: &str) -> Self {
        Self {
            connection_string: connection_string.to_string(),
            table: table.to_string(),
            batch_size: 1000,
        }
    }
}

#[async_trait::async_trait]
impl LoaderTrait for DatabaseLoader {
    async fn load(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        // Would insert into database
        Ok(data.len())
    }

    fn destination_type(&self) -> &str {
        "database"
    }
}

/// File loader.
pub struct FileLoader {
    path: String,
    format: FileFormat,
}

#[derive(Debug, Clone)]
pub enum FileFormat {
    CSV,
    JSON,
    Parquet,
    Avro,
}

impl FileLoader {
    pub fn new(path: &str, format: FileFormat) -> Self {
        Self {
            path: path.to_string(),
            format,
        }
    }
}

#[async_trait::async_trait]
impl LoaderTrait for FileLoader {
    async fn load(&self, data: Vec<serde_json::Value>) -> Result<usize> {
        // Would write to file
        Ok(data.len())
    }

    fn destination_type(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = Loader::new("l-1", "Test Loader", DestinationType::Database);
        assert_eq!(loader.id, "l-1");
        assert_eq!(loader.name, "Test Loader");
    }
}
