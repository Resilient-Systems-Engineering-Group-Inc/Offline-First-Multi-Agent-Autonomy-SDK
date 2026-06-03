//! State migration utilities for version upgrades.

use crate::crdt_map::CrdtMap;
use serde_json::{Value, Map};
use anyhow::{Result, bail};

/// Version identifier for the state schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersion(pub String); // e.g., "1.0", "2.0"

/// A migration that transforms a CRDT map from one version to another.
pub trait Migration: Send + Sync {
    /// Source version (the version before migration).
    fn from_version(&self) -> &SchemaVersion;
    /// Target version (the version after migration).
    fn to_version(&self) -> &SchemaVersion;
    /// Apply the migration to a CRDT map (modify in‑place).
    fn apply(&self, map: &mut CrdtMap) -> Result<()>;
}

/// A simple migration that renames keys according to a mapping.
pub struct KeyRenameMigration {
    from_version: SchemaVersion,
    to_version: SchemaVersion,
    rename_map: Vec<(String, String)>, // (old_key, new_key)
}

impl KeyRenameMigration {
    pub fn new(from: &str, to: &str, rename_map: Vec<(String, String)>) -> Self {
        Self {
            from_version: SchemaVersion(from.to_string()),
            to_version: SchemaVersion(to.to_string()),
            rename_map,
        }
    }
}

impl Migration for KeyRenameMigration {
    fn from_version(&self) -> &SchemaVersion {
        &self.from_version
    }

    fn to_version(&self) -> &SchemaVersion {
        &self.to_version
    }

    fn apply(&self, map: &mut CrdtMap) -> Result<()> {
        // Extract all key‑value pairs with their authors
        let entries: Vec<(String, Value, u64)> = map.to_hashmap_with_authors()
            .into_iter()
            .map(|(k, (v, author))| (k, v, author.0))
            .collect();

        // Build a set of old keys to rename
        let old_keys: std::collections::HashSet<String> = self.rename_map
            .iter()
            .map(|(old, _)| old.clone())
            .collect();

        // Delete all old keys that are being renamed
        for (key, _, author) in &entries {
            if old_keys.contains(key) {
                map.delete(key, AgentId(*author));
            }
        }

        // Re-insert with new keys
        for (key, value, author) in &entries {
            if let Some((_, new_key)) = self.rename_map.iter().find(|(old, _)| old == key) {
                map.set(new_key, value.clone(), AgentId(*author));
            }
        }

        Ok(())
    }
}

use common::types::AgentId;

/// A migration that transforms values using a custom function.
pub struct ValueTransformMigration {
    from_version: SchemaVersion,
    to_version: SchemaVersion,
    transform: Box<dyn Fn(&str, Value) -> Result<Value> + Send + Sync>,
}

impl ValueTransformMigration {
    pub fn new<F>(from: &str, to: &str, transform: F) -> Self
    where
        F: Fn(&str, Value) -> Result<Value> + Send + Sync + 'static,
    {
        Self {
            from_version: SchemaVersion(from.to_string()),
            to_version: SchemaVersion(to.to_string()),
            transform: Box::new(transform),
        }
    }
}

impl Migration for ValueTransformMigration {
    fn from_version(&self) -> &SchemaVersion {
        &self.from_version
    }

    fn to_version(&self) -> &SchemaVersion {
        &self.to_version
    }

    fn apply(&self, map: &mut CrdtMap) -> Result<()> {
        // Extract all key‑value pairs with their authors
        let entries: Vec<(String, Value, u64)> = map.to_hashmap_with_authors()
            .into_iter()
            .map(|(k, (v, author))| (k, v, author.0))
            .collect();

        // Delete all existing entries
        for (key, _, author) in &entries {
            map.delete(key, AgentId(*author));
        }

        // Re-insert with transformed values
        for (key, value, author) in &entries {
            let new_value = (self.transform)(key, value)?;
            map.set(key, new_value, AgentId(*author));
        }

        Ok(())
    }
}

/// Migration manager that applies a sequence of migrations.
pub struct MigrationManager {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationManager {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    pub fn add_migration<M: Migration + 'static>(&mut self, migration: M) {
        self.migrations.push(Box::new(migration));
    }

    /// Migrate a map from a given version to a target version.
    /// Returns the new version after migration.
    pub fn migrate(&self, map: &mut CrdtMap, current_version: &SchemaVersion, target_version: &SchemaVersion) -> Result<SchemaVersion> {
        let mut version = current_version.clone();
        // Find a path of migrations (simple linear search).
        while &version != target_version {
            let migration = self.migrations.iter()
                .find(|m| m.from_version() == &version && m.to_version() == target_version)
                .or_else(|| {
                    // Try to find any migration that starts from current version (step‑by‑step).
                    self.migrations.iter()
                        .find(|m| m.from_version() == &version)
                })
                .ok_or_else(|| anyhow::anyhow!("No migration path from {} to {}", version.0, target_version.0))?;
            migration.apply(map)?;
            version = migration.to_version().clone();
        }
        Ok(version)
    }
}

/// Default migration manager with known migrations.
pub fn default_migration_manager() -> MigrationManager {
    let mut manager = MigrationManager::new();
    // Example: rename "cpu_usage" to "cpu_percent"
    manager.add_migration(KeyRenameMigration::new(
        "1.0",
        "1.1",
        vec![("cpu_usage".to_string(), "cpu_percent".to_string())],
    ));
    manager
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_key_rename_migration() {
        let mut map = CrdtMap::new();
        map.set("cpu_usage", json!(85.0), AgentId(1));
        map.set("memory_usage", json!(1024), AgentId(1));

        let migration = KeyRenameMigration::new(
            "1.0",
            "1.1",
            vec![("cpu_usage".to_string(), "cpu_percent".to_string())],
        );

        let result = migration.apply(&mut map);
        assert!(result.is_ok());

        // Old key should be gone
        let old: Option<serde_json::Value> = map.get("cpu_usage");
        assert!(old.is_none());

        // New key should exist with same value
        let new: Option<serde_json::Value> = map.get("cpu_percent");
        assert!(new.is_some());
        assert_eq!(new.unwrap(), json!(85.0));

        // Unrelated key should remain
        let mem: Option<serde_json::Value> = map.get("memory_usage");
        assert!(mem.is_some());
    }

    #[test]
    fn test_value_transform_migration() {
        let mut map = CrdtMap::new();
        map.set("temperature", json!(30.0), AgentId(1));

        // Convert Celsius to Fahrenheit
        let migration = ValueTransformMigration::new(
            "1.0",
            "2.0",
            |key: &str, value: Value| -> Result<Value> {
                if key == "temperature" {
                    if let Some(celsius) = value.as_f64() {
                        let fahrenheit = celsius * 9.0 / 5.0 + 32.0;
                        Ok(json!(fahrenheit))
                    } else {
                        Ok(value)
                    }
                } else {
                    Ok(value)
                }
            },
        );

        let result = migration.apply(&mut map);
        assert!(result.is_ok());

        let temp: Option<serde_json::Value> = map.get("temperature");
        assert!(temp.is_some());
        assert_eq!(temp.unwrap(), json!(86.0)); // 30°C = 86°F
    }

    #[test]
    fn test_migration_manager() {
        let mut map = CrdtMap::new();
        map.set("cpu_usage", json!(85.0), AgentId(1));

        let manager = default_migration_manager();
        let result = manager.migrate(
            &mut map,
            &SchemaVersion("1.0".to_string()),
            &SchemaVersion("1.1".to_string()),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SchemaVersion("1.1".to_string()));

        // Verify the rename happened
        let old: Option<serde_json::Value> = map.get("cpu_usage");
        assert!(old.is_none());
        let new: Option<serde_json::Value> = map.get("cpu_percent");
        assert!(new.is_some());
    }
}