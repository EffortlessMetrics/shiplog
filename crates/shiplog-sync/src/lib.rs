//! Synchronization utilities for remote sources.
//!
//! This crate provides functionality for synchronizing data from remote sources.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Sync status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub source: String,
    pub interval_seconds: u64,
    pub retry_count: u32,
    pub last_sync: Option<DateTime<Utc>>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            source: String::new(),
            interval_seconds: 3600,
            retry_count: 3,
            last_sync: None,
        }
    }
}

/// Sync state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub status: SyncStatus,
    pub config: SyncConfig,
    pub items_synced: usize,
}

impl SyncState {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            status: SyncStatus::Pending,
            config,
            items_synced: 0,
        }
    }

    /// Mark sync as started
    pub fn start(&mut self) {
        self.status = SyncStatus::InProgress;
    }

    /// Mark sync as completed
    pub fn complete(&mut self, items_synced: usize) {
        self.items_synced = items_synced;
        self.status = SyncStatus::Completed;
        self.config.last_sync = Some(Utc::now());
    }

    /// Mark sync as failed
    pub fn fail(&mut self, error: String) {
        self.status = SyncStatus::Failed(error);
    }

    /// Check if sync is needed
    pub fn needs_sync(&self) -> bool {
        match self.status {
            SyncStatus::Pending => true,
            SyncStatus::Completed => {
                if let Some(last) = self.config.last_sync {
                    let elapsed = Utc::now().signed_duration_since(last);
                    elapsed.num_seconds() >= self.config.interval_seconds as i64
                } else {
                    true
                }
            }
            SyncStatus::Failed(_) => true,
            SyncStatus::InProgress => false,
        }
    }
}

/// Sync result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub items_synced: usize,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
}

impl SyncResult {
    pub fn success(items_synced: usize) -> Self {
        Self {
            success: true,
            items_synced,
            timestamp: Utc::now(),
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            items_synced: 0,
            timestamp: Utc::now(),
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_creation() {
        let config = SyncConfig::default();
        let state = SyncState::new(config);
        
        assert_eq!(state.status, SyncStatus::Pending);
        assert_eq!(state.items_synced, 0);
    }

    #[test]
    fn test_sync_state_start() {
        let config = SyncConfig::default();
        let mut state = SyncState::new(config);
        
        state.start();
        assert_eq!(state.status, SyncStatus::InProgress);
    }

    #[test]
    fn test_sync_state_complete() {
        let config = SyncConfig::default();
        let mut state = SyncState::new(config);
        
        state.start();
        state.complete(42);
        
        assert_eq!(state.items_synced, 42);
        assert_eq!(state.status, SyncStatus::Completed);
        assert!(state.config.last_sync.is_some());
    }

    #[test]
    fn test_sync_state_fail() {
        let config = SyncConfig::default();
        let mut state = SyncState::new(config);
        
        state.fail("Network error".to_string());
        
        assert!(matches!(state.status, SyncStatus::Failed(msg) if msg == "Network error"));
    }

    #[test]
    fn test_needs_sync_pending() {
        let config = SyncConfig::default();
        let state = SyncState::new(config);
        
        assert!(state.needs_sync());
    }

    #[test]
    fn test_needs_sync_completed_interval_elapsed() {
        let config = SyncConfig {
            source: "test".to_string(),
            interval_seconds: 3600,
            retry_count: 3,
            // Last sync was 2 hours ago (past 3600 second interval)
            last_sync: Some(Utc::now() - chrono::Duration::hours(2)),
        };
        let state = SyncState::new(config);
        
        // Interval elapsed, sync needed
        assert!(state.needs_sync());
    }

    #[test]
    fn test_needs_sync_in_progress() {
        let config = SyncConfig::default();
        let mut state = SyncState::new(config);
        
        state.start();
        assert!(!state.needs_sync());
    }

    #[test]
    fn test_needs_sync_completed_no_interval() {
        // When interval is 0, consider it as immediate re-sync needed
        let config = SyncConfig {
            source: "test".to_string(),
            interval_seconds: 0,
            retry_count: 3,
            last_sync: Some(Utc::now()),
        };
        let state = SyncState::new(config);
        
        // With interval 0, always need to sync
        assert!(state.needs_sync());
    }

    #[test]
    fn test_sync_result_success() {
        let result = SyncResult::success(100);
        
        assert!(result.success);
        assert_eq!(result.items_synced, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_sync_result_failure() {
        let result = SyncResult::failure("Timeout".to_string());
        
        assert!(!result.success);
        assert_eq!(result.items_synced, 0);
        assert!(result.error.is_some());
    }
}
