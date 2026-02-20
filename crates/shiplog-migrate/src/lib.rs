//! Schema migration utilities for shiplog.

use serde::{Deserialize, Serialize};

/// Schema version
pub type SchemaVersion = u32;

/// Migration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// A migration step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Migration version (from)
    pub from_version: SchemaVersion,
    /// Migration version (to)
    pub to_version: SchemaVersion,
    /// Migration name
    pub name: String,
    /// Migration description
    pub description: String,
    /// Up migration function (JSON string for portability)
    pub up_script: String,
    /// Down migration function (for rollback)
    pub down_script: String,
}

impl Migration {
    /// Create a new migration
    pub fn new(from: SchemaVersion, to: SchemaVersion, name: impl Into<String>) -> Self {
        Self {
            from_version: from,
            to_version: to,
            name: name.into(),
            description: String::new(),
            up_script: String::new(),
            down_script: String::new(),
        }
    }
    
    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
    
    /// Set the up migration script
    pub fn with_up_script(mut self, script: impl Into<String>) -> Self {
        self.up_script = script.into();
        self
    }
    
    /// Set the down migration script
    pub fn with_down_script(mut self, script: impl Into<String>) -> Self {
        self.down_script = script.into();
        self
    }
}

/// Migration history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration version
    pub version: SchemaVersion,
    /// Migration name
    pub name: String,
    /// When migration was applied
    pub applied_at: chrono::DateTime<chrono::Utc>,
    /// Migration status
    pub status: MigrationStatus,
}

/// Migration runner
pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new(migrations: Vec<Migration>) -> Self {
        Self { migrations }
    }
    
    /// Get the latest version
    pub fn latest_version(&self) -> SchemaVersion {
        self.migrations
            .iter()
            .map(|m| m.to_version)
            .max()
            .unwrap_or(0)
    }
    
    /// Get pending migrations from a given version
    pub fn get_pending_migrations(&self, from_version: SchemaVersion) -> Vec<&Migration> {
        self.migrations
            .iter()
            .filter(|m| m.from_version >= from_version)
            .collect()
    }
    
    /// Apply a migration (simulated - actual implementation would execute the script)
    pub fn apply_migration(&self, migration: &Migration) -> anyhow::Result<MigrationRecord> {
        // In a real implementation, this would execute the up_script
        // For now, we simulate the migration
        Ok(MigrationRecord {
            version: migration.to_version,
            name: migration.name.clone(),
            applied_at: chrono::Utc::now(),
            status: MigrationStatus::Completed,
        })
    }
}

/// Schema state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaState {
    /// Current schema version
    pub current_version: SchemaVersion,
    /// Migration history
    pub history: Vec<MigrationRecord>,
}

impl Default for SchemaState {
    fn default() -> Self {
        Self {
            current_version: 0,
            history: Vec::new(),
        }
    }
}

impl SchemaState {
    /// Create a new schema state
    pub fn new(version: SchemaVersion) -> Self {
        Self {
            current_version: version,
            history: Vec::new(),
        }
    }
    
    /// Record a migration
    pub fn record_migration(&mut self, record: MigrationRecord) {
        self.current_version = record.version;
        self.history.push(record);
    }
    
    /// Check if migration is needed
    pub fn needs_migration(&self, target_version: SchemaVersion) -> bool {
        self.current_version < target_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn migration_creation() {
        let migration = Migration::new(1, 2, "add_workstream_field")
            .with_description("Add workstream field to events")
            .with_up_script(r#"{"add_field": "workstream_id"}"#)
            .with_down_script(r#"{"remove_field": "workstream_id"}"#);
        
        assert_eq!(migration.from_version, 1);
        assert_eq!(migration.to_version, 2);
        assert_eq!(migration.name, "add_workstream_field");
    }

    #[test]
    fn migration_runner_latest_version() {
        let migrations = vec![
            Migration::new(1, 2, "v1_to_v2"),
            Migration::new(2, 3, "v2_to_v3"),
            Migration::new(3, 4, "v3_to_v4"),
        ];
        
        let runner = MigrationRunner::new(migrations);
        assert_eq!(runner.latest_version(), 4);
    }

    #[test]
    fn migration_runner_pending_migrations() {
        let migrations = vec![
            Migration::new(1, 2, "v1_to_v2"),
            Migration::new(2, 3, "v2_to_v3"),
            Migration::new(3, 4, "v3_to_v4"),
        ];
        
        let runner = MigrationRunner::new(migrations);
        let pending = runner.get_pending_migrations(2);
        
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].to_version, 3);
        assert_eq!(pending[1].to_version, 4);
    }

    #[test]
    fn schema_state_default() {
        let state = SchemaState::default();
        assert_eq!(state.current_version, 0);
        assert!(state.history.is_empty());
    }

    #[test]
    fn schema_state_needs_migration() {
        let state = SchemaState::new(2);
        assert!(state.needs_migration(3));
        assert!(!state.needs_migration(2));
        assert!(!state.needs_migration(1));
    }

    #[test]
    fn schema_state_record_migration() {
        let mut state = SchemaState::new(1);
        
        let record = MigrationRecord {
            version: 2,
            name: "v1_to_v2".to_string(),
            applied_at: chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            status: MigrationStatus::Completed,
        };
        
        state.record_migration(record);
        
        assert_eq!(state.current_version, 2);
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].name, "v1_to_v2");
    }

    #[test]
    fn migration_apply() {
        let migration = Migration::new(1, 2, "test_migration");
        let runner = MigrationRunner::new(vec![migration]);
        
        let result = runner.apply_migration(&runner.migrations[0]);
        
        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.version, 2);
        assert_eq!(record.status, MigrationStatus::Completed);
    }
}
