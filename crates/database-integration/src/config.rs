//! Configuration for database connections.

use serde::{Deserialize, Serialize};

/// Database type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseType {
    /// SQLite (file‑based).
    Sqlite,
    /// PostgreSQL.
    Postgres,
    /// Redis (key‑value store).
    Redis,
    /// MongoDB (document store).
    Mongo,
}

/// General database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type.
    pub db_type: DatabaseType,
    /// Connection string (URL, path, etc.).
    pub connection_string: String,
    /// Connection pool size.
    pub pool_size: u32,
    /// Timeout in seconds.
    pub timeout_secs: u64,
    /// Enable TLS (if applicable).
    pub tls: bool,
    /// Additional options (key‑value pairs).
    pub options: Vec<(String, String)>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: DatabaseType::Sqlite,
            connection_string: "sqlite::memory:".to_string(),
            pool_size: 5,
            timeout_secs: 30,
            tls: false,
            options: Vec::new(),
        }
    }
}

/// SQLite‑specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Path to database file (":memory:" for in‑memory).
    pub path: String,
    /// Enable foreign keys.
    pub foreign_keys: bool,
    /// Journal mode (WAL, DELETE, etc.).
    pub journal_mode: String,
    /// Synchronous setting.
    pub synchronous: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: ":memory:".to_string(),
            foreign_keys: true,
            journal_mode: "WAL".to_string(),
            synchronous: "NORMAL".to_string(),
        }
    }
}

/// PostgreSQL‑specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Hostname.
    pub host: String,
    /// Port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
    /// SSL mode.
    pub ssl_mode: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "postgres".to_string(),
            username: "postgres".to_string(),
            password: "".to_string(),
            ssl_mode: "prefer".to_string(),
        }
    }
}

/// Redis‑specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL (e.g., "redis://localhost:6379").
    pub url: String,
    /// Database number.
    pub db: u8,
    /// Password (optional).
    pub password: Option<String>,
    /// Use TLS.
    pub tls: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            db: 0,
            password: None,
            tls: false,
        }
    }
}

/// MongoDB‑specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoConfig {
    /// Connection URI.
    pub uri: String,
    /// Database name.
    pub database: String,
    /// Collection name.
    pub collection: String,
    /// Use TLS.
    pub tls: bool,
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            uri: "mongodb://localhost:27017".to_string(),
            database: "test".to_string(),
            collection: "documents".to_string(),
            tls: false,
        }
    }
}