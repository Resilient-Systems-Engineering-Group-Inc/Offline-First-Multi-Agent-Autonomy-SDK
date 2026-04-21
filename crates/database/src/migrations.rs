//! Database migrations.

use sqlx::Pool;
use anyhow::Result;
use tracing::info;

/// Run all migrations.
pub async fn migrate(pool: &Pool) -> Result<()> {
    info!("Running database migrations...");

    // Create tables
    create_tasks_table(pool).await?;
    create_workflows_table(pool).await?;
    create_workflow_instances_table(pool).await?;
    create_agents_table(pool).await?;
    create_audit_logs_table(pool).await?;
    create_users_table(pool).await?;
    create_api_tokens_table(pool).await?;
    
    // Create indexes
    create_indexes(pool).await?;

    info!("All migrations completed successfully");
    Ok(())
}

/// Create tasks table.
async fn create_tasks_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            priority INTEGER NOT NULL DEFAULT 100,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            started_at TIMESTAMP,
            completed_at TIMESTAMP,
            assigned_agent TEXT,
            workflow_instance_id TEXT,
            parameters TEXT NOT NULL DEFAULT '{}',
            required_capabilities TEXT NOT NULL DEFAULT '[]',
            dependencies TEXT NOT NULL DEFAULT '[]',
            result TEXT,
            error_message TEXT,
            retry_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (workflow_instance_id) REFERENCES workflow_instances(id) ON DELETE SET NULL
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create workflows table.
async fn create_workflows_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            version TEXT NOT NULL DEFAULT '1.0.0',
            yaml_definition TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_active INTEGER NOT NULL DEFAULT 1,
            metadata TEXT NOT NULL DEFAULT '{}'
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create workflow_instances table.
async fn create_workflow_instances_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS workflow_instances (
            id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            progress REAL NOT NULL DEFAULT 0.0,
            started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            completed_at TIMESTAMP,
            parameters TEXT NOT NULL DEFAULT '{}',
            output TEXT NOT NULL DEFAULT '{}',
            error_message TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create agents table.
async fn create_agents_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'offline',
            capabilities TEXT NOT NULL DEFAULT '[]',
            resources TEXT NOT NULL DEFAULT '{}',
            connected_peers INTEGER NOT NULL DEFAULT 0,
            active_tasks TEXT NOT NULL DEFAULT '[]',
            last_heartbeat TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            metadata TEXT NOT NULL DEFAULT '{}'
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create audit_logs table.
async fn create_audit_logs_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            actor_id TEXT,
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            old_value TEXT,
            new_value TEXT,
            metadata TEXT NOT NULL DEFAULT '{}',
            ip_address TEXT
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create users table.
async fn create_users_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            email TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_login TIMESTAMP
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create api_tokens table.
async fn create_api_tokens_table(pool: &Pool) -> Result<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS api_tokens (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token_hash TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            expires_at TIMESTAMP,
            last_used_at TIMESTAMP,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            scopes TEXT NOT NULL DEFAULT '[]',
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#;

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

/// Create indexes for better performance.
async fn create_indexes(pool: &Pool) -> Result<()> {
    info!("Creating database indexes...");

    let indexes = vec![
        // Task indexes
        "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority DESC)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_workflow ON tasks(workflow_instance_id)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_assigned_agent ON tasks(assigned_agent)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at DESC)",
        
        // Workflow indexes
        "CREATE INDEX IF NOT EXISTS idx_workflows_active ON workflows(is_active)",
        "CREATE INDEX IF NOT EXISTS idx_workflows_name ON workflows(name)",
        
        // Workflow instance indexes
        "CREATE INDEX IF NOT EXISTS idx_workflow_instances_status ON workflow_instances(status)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_instances_workflow ON workflow_instances(workflow_id)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_instances_started ON workflow_instances(started_at DESC)",
        
        // Agent indexes
        "CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status)",
        "CREATE INDEX IF NOT EXISTS idx_agents_heartbeat ON agents(last_heartbeat DESC)",
        
        // Audit log indexes
        "CREATE INDEX IF NOT EXISTS idx_audit_logs_entity ON audit_logs(entity_type, entity_id)",
        "CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC)",
        "CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id)",
        
        // User indexes
        "CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)",
        "CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active)",
        
        // API token indexes
        "CREATE INDEX IF NOT EXISTS idx_api_tokens_user ON api_tokens(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash)",
        "CREATE INDEX IF NOT EXISTS idx_api_tokens_active ON api_tokens(is_active)",
    ];

    for index_query in indexes {
        sqlx::query(index_query).execute(pool).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_migrations() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        
        migrate(&pool).await.unwrap();
        
        // Verify tables exist
        let query = "SELECT name FROM sqlite_master WHERE type='table'";
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        
        assert!(rows.len() >= 7); // All tables should exist
    }
}
