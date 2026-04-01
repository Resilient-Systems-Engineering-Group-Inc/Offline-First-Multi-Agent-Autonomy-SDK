//! Integration with SQL and NoSQL databases for offline‑first multi‑agent systems.
//!
//! This crate provides a unified interface to various databases (SQLite, PostgreSQL,
//! Redis, MongoDB) with connection pooling, migrations, and CRUD operations.
//!
//! # Quick Start
//!
//! ```no_run
//! use database_integration::{Database, SqliteDatabase, DatabaseConfig, DatabaseType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DatabaseConfig {
//!         db_type: DatabaseType::Sqlite,
//!         connection_string: "sqlite::memory:".to_string(),
//!         ..Default::default()
//!     };
//!     let db = SqliteDatabase::connect(config).await?;
//!     db.execute("CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, value TEXT)").await?;
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod sqlite;
pub mod postgres;
pub mod redis;
pub mod mongo;

pub use config::*;
pub use error::*;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Unified database trait.
#[async_trait]
pub trait Database: Send + Sync {
    /// Execute a raw query (no results).
    async fn execute(&self, query: &str) -> Result<()>;

    /// Execute a query that returns rows.
    async fn query(&self, query: &str) -> Result<Vec<serde_json::Value>>;

    /// Insert a record into a table.
    async fn insert(&self, table: &str, record: &serde_json::Value) -> Result<u64>;

    /// Update records matching a condition.
    async fn update(&self, table: &str, condition: &str, updates: &serde_json::Value) -> Result<u64>;

    /// Delete records matching a condition.
    async fn delete(&self, table: &str, condition: &str) -> Result<u64>;

    /// Select records with optional condition.
    async fn select(&self, table: &str, condition: Option<&str>) -> Result<Vec<serde_json::Value>>;

    /// Begin a transaction.
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;

    /// Check if the database is connected.
    async fn ping(&self) -> Result<bool>;
}

/// Transaction handle.
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Commit the transaction.
    async fn commit(self: Box<Self>) -> Result<()>;
    /// Rollback the transaction.
    async fn rollback(self: Box<Self>) -> Result<()>;
}

/// Connect to a database based on configuration.
pub async fn connect(config: DatabaseConfig) -> Result<Box<dyn Database>> {
    match config.db_type {
        DatabaseType::Sqlite => {
            let db = sqlite::SqliteDatabase::connect(config).await?;
            Ok(Box::new(db))
        }
        DatabaseType::Postgres => {
            let db = postgres::PostgresDatabase::connect(config).await?;
            Ok(Box::new(db))
        }
        DatabaseType::Redis => {
            let db = redis::RedisDatabase::connect(config).await?;
            Ok(Box::new(db))
        }
        DatabaseType::Mongo => {
            let db = mongo::MongoDatabase::connect(config).await?;
            Ok(Box::new(db))
        }
    }
}