//! SQLite database integration.

use crate::config::DatabaseConfig;
use crate::error::{DatabaseError, Result};
use crate::{Database, Transaction};
use async_trait::async_trait;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;

/// SQLite database connection.
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    /// Connect to a SQLite database.
    pub async fn connect(config: DatabaseConfig) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.pool_size)
            .connect(&config.connection_string)
            .await
            .map_err(DatabaseError::Sql)?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn execute(&self, query: &str) -> Result<()> {
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Sql)?;
        Ok(())
    }

    async fn query(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::Sql)?;
        let mut results = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for (i, column) in row.columns().iter().enumerate() {
                let value: serde_json::Value = row.try_get(i).unwrap_or(serde_json::Value::Null);
                map.insert(column.name().to_string(), value);
            }
            results.push(serde_json::Value::Object(map));
        }
        Ok(results)
    }

    async fn insert(&self, table: &str, record: &serde_json::Value) -> Result<u64> {
        let keys: Vec<String> = record
            .as_object()
            .ok_or_else(|| DatabaseError::Query("Record must be an object".to_string()))?
            .keys()
            .cloned()
            .collect();
        let placeholders: Vec<String> = (0..keys.len()).map(|i| format!("?{}", i + 1)).collect();
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            keys.join(", "),
            placeholders.join(", ")
        );
        let mut q = sqlx::query(&query);
        for key in &keys {
            let val = record.get(key).unwrap_or(&serde_json::Value::Null);
            let val_str = serde_json::to_string(val).map_err(DatabaseError::Serialization)?;
            q = q.bind(val_str);
        }
        let result = q.execute(&self.pool).await.map_err(DatabaseError::Sql)?;
        Ok(result.last_insert_rowid() as u64)
    }

    async fn update(&self, table: &str, condition: &str, updates: &serde_json::Value) -> Result<u64> {
        let updates_map = updates
            .as_object()
            .ok_or_else(|| DatabaseError::Query("Updates must be an object".to_string()))?;
        let set_clause: Vec<String> = updates_map
            .keys()
            .map(|k| format!("{} = ?", k))
            .collect();
        let query = format!(
            "UPDATE {} SET {} WHERE {}",
            table,
            set_clause.join(", "),
            condition
        );
        let mut q = sqlx::query(&query);
        for key in updates_map.keys() {
            let val = updates_map.get(key).unwrap_or(&serde_json::Value::Null);
            let val_str = serde_json::to_string(val).map_err(DatabaseError::Serialization)?;
            q = q.bind(val_str);
        }
        let result = q.execute(&self.pool).await.map_err(DatabaseError::Sql)?;
        Ok(result.rows_affected())
    }

    async fn delete(&self, table: &str, condition: &str) -> Result<u64> {
        let query = format!("DELETE FROM {} WHERE {}", table, condition);
        let result = sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Sql)?;
        Ok(result.rows_affected())
    }

    async fn select(&self, table: &str, condition: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let query = match condition {
            Some(cond) => format!("SELECT * FROM {} WHERE {}", table, cond),
            None => format!("SELECT * FROM {}", table),
        };
        self.query(&query).await
    }

    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>> {
        let tx = self.pool.begin().await.map_err(DatabaseError::Sql)?;
        Ok(Box::new(SqliteTransaction { tx }))
    }

    async fn ping(&self) -> Result<bool> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::Sql(e))
            .map(|_| true)
    }
}

struct SqliteTransaction {
    tx: sqlx::Transaction<'static, sqlx::Sqlite>,
}

#[async_trait]
impl Transaction for SqliteTransaction {
    async fn commit(mut self: Box<Self>) -> Result<()> {
        self.tx.commit().await.map_err(DatabaseError::Sql)?;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> Result<()> {
        self.tx.rollback().await.map_err(DatabaseError::Sql)?;
        Ok(())
    }
}