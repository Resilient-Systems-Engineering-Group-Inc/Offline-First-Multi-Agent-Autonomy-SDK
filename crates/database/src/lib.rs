//! Database persistence layer for the SDK.
//!
//! Provides:
//! - SQLite for local/embedded deployments
//! - PostgreSQL for distributed deployments
//! - Workflow state persistence
//! - Task history
//! - Agent state
//! - Audit logs

pub mod models;
pub mod repository;
pub mod migrations;

use sqlx::{Pool, Row};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Duration;
use tracing::{info, error};
use anyhow::Result;

pub use models::*;
pub use repository::*;

/// Database configuration.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:sdk.db?mode=rwc".to_string(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
        }
    }
}

impl DatabaseConfig {
    /// Create SQLite config.
    pub fn sqlite(path: &str) -> Self {
        Self {
            url: format!("sqlite:{}?mode=rwc", path),
            ..Default::default()
        }
    }

    /// Create PostgreSQL config.
    pub fn postgres(host: &str, database: &str, user: &str, password: &str) -> Self {
        Self {
            url: format!("postgres://{}:{}@{}/{}", user, password, host, database),
            ..Default::default()
        }
    }
}

/// Database connection pool.
#[derive(Clone)]
pub struct Database {
    pool: Pool,
}

impl Database {
    /// Create new database connection.
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        info!("Connecting to database: {}", config.url);

        let is_sqlite = config.url.starts_with("sqlite:");

        let pool = if is_sqlite {
            SqlitePoolOptions::new()
                .max_connections(config.max_connections)
                .min_connections(config.min_connections)
                .acquire_timeout(config.connect_timeout)
                .connect(&config.url)
                .await?
        } else {
            PgPoolOptions::new()
                .max_connections(config.max_connections)
                .min_connections(config.min_connections)
                .acquire_timeout(config.connect_timeout)
                .idle_timeout(config.idle_timeout)
                .connect(&config.url)
                .await?
        };

        let db = Self { pool };
        
        // Run migrations
        db.run_migrations().await?;
        
        Ok(db)
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");
        
        migrations::migrate(&self.pool).await?;
        
        info!("Database migrations completed");
        Ok(())
    }

    /// Get connection pool.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Check database health.
    pub async fn health(&self) -> Result<bool> {
        let query = if self.is_sqlite() {
            "SELECT 1"
        } else {
            "SELECT version()"
        };

        match sqlx::query(query).fetch_one(&self.pool).await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Check if using SQLite.
    pub fn is_sqlite(&self) -> bool {
        // Implementation would check pool type
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_connection() {
        let config = DatabaseConfig::sqlite(":memory:");
        let db = Database::new(config).await.unwrap();
        
        assert!(db.health().await.unwrap());
    }

    #[tokio::test]
    async fn test_repository_crud() {
        let config = DatabaseConfig::sqlite(":memory:");
        let db = Database::new(config).await.unwrap();
        
        let mut task_repo = TaskRepository::new(db.pool());
        
        // Create task
        let task = TaskModel {
            id: uuid::Uuid::new_v4().to_string(),
            description: "Test task".to_string(),
            status: "pending".to_string(),
            priority: 100,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
            assigned_agent: None,
            workflow_instance_id: None,
            parameters: serde_json::json!({}),
        };
        
        let created = task_repo.create(&task).await.unwrap();
        assert!(!created.id.is_empty());
        
        // Get task
        let retrieved = task_repo.get(&created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.description, "Test task");
        
        // Update task
        let mut updated_task = retrieved.clone();
        updated_task.status = "completed".to_string();
        task_repo.update(&updated_task).await.unwrap();
        
        // Verify update
        let final_task = task_repo.get(&created.id).await.unwrap().unwrap();
        assert_eq!(final_task.status, "completed");
        
        // Delete task
        task_repo.delete(&created.id).await.unwrap();
        
        // Verify deletion
        let deleted = task_repo.get(&created.id).await.unwrap();
        assert!(deleted.is_none());
    }
}
