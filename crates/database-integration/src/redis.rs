//! Redis database integration.

use crate::config::DatabaseConfig;
use crate::error::{DatabaseError, Result};
use crate::{Database, Transaction};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use std::sync::Arc;

/// Redis database connection.
pub struct RedisDatabase {
    conn: ConnectionManager,
}

impl RedisDatabase {
    /// Connect to a Redis database.
    pub async fn connect(config: DatabaseConfig) -> Result<Self> {
        let client = redis::Client::open(config.connection_string)
            .map_err(DatabaseError::Redis)?;
        let conn = client
            .get_tokio_connection_manager()
            .await
            .map_err(DatabaseError::Redis)?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl Database for RedisDatabase {
    async fn execute(&self, query: &str) -> Result<()> {
        // Redis does not have a generic execute; we treat query as a Redis command.
        // For simplicity, we split by spaces and send as command.
        let parts: Vec<&str> = query.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        let mut cmd = redis::cmd(parts[0]);
        for arg in &parts[1..] {
            cmd.arg(*arg);
        }
        cmd.query_async(&mut self.conn.clone())
            .await
            .map_err(DatabaseError::Redis)?;
        Ok(())
    }

    async fn query(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        // For Redis, a query could be a GET or SCAN etc. We'll just return empty.
        // This is a placeholder.
        Ok(Vec::new())
    }

    async fn insert(&self, table: &str, record: &serde_json::Value) -> Result<u64> {
        // Treat table as key prefix.
        let key = format!("{}:{}", table, uuid::Uuid::new_v4());
        let value = serde_json::to_string(record).map_err(DatabaseError::Serialization)?;
        let _: () = self
            .conn
            .clone()
            .set(&key, value)
            .await
            .map_err(DatabaseError::Redis)?;
        Ok(1)
    }

    async fn update(&self, table: &str, condition: &str, updates: &serde_json::Value) -> Result<u64> {
        // Simplified: find keys matching pattern and update.
        let pattern = format!("{}:*", table);
        let keys: Vec<String> = self
            .conn
            .clone()
            .scan_match(&pattern)
            .await
            .map_err(DatabaseError::Redis)?;
        let mut updated = 0;
        for key in keys {
            // For each key, merge updates.
            let old: Option<String> = self.conn.clone().get(&key).await.map_err(DatabaseError::Redis)?;
            if let Some(old_str) = old {
                let mut old_val: serde_json::Value =
                    serde_json::from_str(&old_str).map_err(DatabaseError::Serialization)?;
                if let (Some(old_obj), Some(updates_obj)) = (old_val.as_object_mut(), updates.as_object()) {
                    for (k, v) in updates_obj {
                        old_obj.insert(k.clone(), v.clone());
                    }
                    let new_str = serde_json::to_string(&old_val).map_err(DatabaseError::Serialization)?;
                    let _: () = self.conn.clone().set(&key, new_str).await.map_err(DatabaseError::Redis)?;
                    updated += 1;
                }
            }
        }
        Ok(updated)
    }

    async fn delete(&self, table: &str, condition: &str) -> Result<u64> {
        // Delete keys matching pattern.
        let pattern = format!("{}:*", table);
        let keys: Vec<String> = self
            .conn
            .clone()
            .scan_match(&pattern)
            .await
            .map_err(DatabaseError::Redis)?;
        let mut deleted = 0;
        for key in keys {
            let _: () = self.conn.clone().del(&key).await.map_err(DatabaseError::Redis)?;
            deleted += 1;
        }
        Ok(deleted)
    }

    async fn select(&self, table: &str, condition: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let pattern = format!("{}:*", table);
        let keys: Vec<String> = self
            .conn
            .clone()
            .scan_match(&pattern)
            .await
            .map_err(DatabaseError::Redis)?;
        let mut results = Vec::new();
        for key in keys {
            let val: Option<String> = self.conn.clone().get(&key).await.map_err(DatabaseError::Redis)?;
            if let Some(val_str) = val {
                let json: serde_json::Value =
                    serde_json::from_str(&val_str).map_err(DatabaseError::Serialization)?;
                results.push(json);
            }
        }
        Ok(results)
    }

    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>> {
        // Redis transactions (MULTI/EXEC) could be implemented.
        // For now, return a dummy transaction.
        Ok(Box::new(RedisTransaction {
            conn: self.conn.clone(),
        }))
    }

    async fn ping(&self) -> Result<bool> {
        let _: String = redis::cmd("PING")
            .query_async(&mut self.conn.clone())
            .await
            .map_err(DatabaseError::Redis)?;
        Ok(true)
    }
}

struct RedisTransaction {
    conn: ConnectionManager,
}

#[async_trait]
impl Transaction for RedisTransaction {
    async fn commit(self: Box<Self>) -> Result<()> {
        // Redis transaction commit (EXEC) would go here.
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<()> {
        // Redis transaction discard (DISCARD) would go here.
        Ok(())
    }
}