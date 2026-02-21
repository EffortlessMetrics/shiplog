//! Shared state utilities for shiplog.
//!
//! This crate provides utilities for managing shared state across async tasks.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A guard that provides read access to shared state.
#[derive(Debug)]
pub struct ReadGuard<'a, T> {
    guard: tokio::sync::RwLockReadGuard<'a, T>,
}

impl<T> std::ops::Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

/// A guard that provides write access to shared state.
#[derive(Debug)]
pub struct WriteGuard<'a, T> {
    guard: tokio::sync::RwLockWriteGuard<'a, T>,
}

impl<T> std::ops::Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<T> std::ops::DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

/// Shared state that can be accessed from multiple tasks.
///
/// This is a wrapper around `tokio::sync::RwLock` for convenient async access.
#[derive(Debug)]
pub struct SharedState<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> SharedState<T> {
    /// Create new shared state with the given initial value.
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    /// Get a read guard for the state.
    ///
    /// Multiple read guards can be held concurrently.
    pub async fn read(&self) -> ReadGuard<'_, T> {
        ReadGuard {
            guard: self.inner.read().await,
        }
    }

    /// Get a write guard for the state.
    ///
    /// Only one write guard can be held at a time, and no read guards while held.
    pub async fn write(&self) -> WriteGuard<'_, T> {
        WriteGuard {
            guard: self.inner.write().await,
        }
    }

    /// Get the current value without holding a guard.
    pub async fn get(&self) -> T
    where
        T: Clone,
    {
        self.read().await.clone()
    }

    /// Update the value using a closure.
    pub async fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.write().await;
        f(&mut guard);
    }

    /// Check if the state is the only reference (not shared).
    pub fn is_unique(&self) -> bool {
        Arc::strong_count(&self.inner) == 1
    }
}

impl<T> Clone for SharedState<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Configuration for shared state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedStateConfig {
    /// Initial value for the state.
    pub initial_value: Option<String>,
    /// Whether the state should persist.
    pub persistent: bool,
    /// Path for persistence (if enabled).
    pub persist_path: Option<String>,
}

impl Default for SharedStateConfig {
    fn default() -> Self {
        Self {
            initial_value: None,
            persistent: false,
            persist_path: None,
        }
    }
}

/// A container for multiple shared states.
#[derive(Debug)]
pub struct SharedStateMap {
    states: SharedState<std::collections::HashMap<String, String>>,
}

impl SharedStateMap {
    /// Create a new empty state map.
    pub fn new() -> Self {
        Self {
            states: SharedState::new(std::collections::HashMap::new()),
        }
    }

    /// Insert a value.
    pub async fn insert(&self, key: String, value: String) {
        let mut guard = self.states.write().await;
        guard.insert(key, value);
    }

    /// Get a value.
    pub async fn get(&self, key: &str) -> Option<String> {
        let guard = self.states.read().await;
        guard.get(key).cloned()
    }

    /// Remove a value.
    pub async fn remove(&self, key: &str) -> Option<String> {
        let mut guard = self.states.write().await;
        guard.remove(key)
    }

    /// Check if a key exists.
    pub async fn contains_key(&self, key: &str) -> bool {
        let guard = self.states.read().await;
        guard.contains_key(key)
    }

    /// Get all keys.
    pub async fn keys(&self) -> Vec<String> {
        let guard = self.states.read().await;
        guard.keys().cloned().collect()
    }

    /// Get the number of entries.
    pub async fn len(&self) -> usize {
        let guard = self.states.read().await;
        guard.len()
    }

    /// Check if empty.
    pub async fn is_empty(&self) -> bool {
        let guard = self.states.read().await;
        guard.is_empty()
    }
}

impl Default for SharedStateMap {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedStateMap {
    fn clone(&self) -> Self {
        Self {
            states: self.states.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_state_new() {
        let state = SharedState::new(42);
        assert!(state.is_unique());
    }

    #[tokio::test]
    async fn test_shared_state_read_write() {
        let state = SharedState::new(42);
        
        // Read
        let guard = state.read().await;
        assert_eq!(*guard, 42);
        
        // Write
        drop(guard);
        let mut guard = state.write().await;
        *guard = 100;
        assert_eq!(*guard, 100);
    }

    #[tokio::test]
    async fn test_shared_state_get() {
        let state = SharedState::new(42);
        let value = state.get().await;
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_shared_state_update() {
        let state = SharedState::new(42);
        
        state.update(|v| {
            *v *= 2;
        }).await;
        
        let value = state.get().await;
        assert_eq!(value, 84);
    }

    #[tokio::test]
    async fn test_shared_state_clone() {
        let state = SharedState::new(42);
        let state_clone = state.clone();
        
        assert!(!state.is_unique());
        assert!(!state_clone.is_unique());
        
        let value = state_clone.get().await;
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_shared_state_map() {
        let map = SharedStateMap::new();
        
        map.insert("key1".to_string(), "value1".to_string()).await;
        map.insert("key2".to_string(), "value2".to_string()).await;
        
        assert_eq!(map.len().await, 2);
        assert!(map.contains_key("key1").await);
        assert_eq!(map.get("key1").await, Some("value1".to_string()));
        
        let removed = map.remove("key1").await;
        assert_eq!(removed, Some("value1".to_string()));
        assert!(!map.contains_key("key1").await);
    }

    #[tokio::test]
    async fn test_shared_state_map_keys() {
        let map = SharedStateMap::new();
        
        map.insert("a".to_string(), "1".to_string()).await;
        map.insert("b".to_string(), "2".to_string()).await;
        
        let keys = map.keys().await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
    }
}
