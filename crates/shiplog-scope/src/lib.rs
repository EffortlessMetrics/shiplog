//! Scoped task utilities for shiplog.
//!
//! This crate provides scoped task utilities for managing groups of related tasks.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Configuration for scoped tasks
#[derive(Debug, Clone)]
pub struct ScopeConfig {
    pub max_tasks: usize,
    pub propagate_panic: bool,
    pub await_all: bool,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            max_tasks: 100,
            propagate_panic: false,
            await_all: true,
        }
    }
}

/// Builder for scope configuration
#[derive(Debug)]
pub struct ScopeBuilder {
    config: ScopeConfig,
}

impl ScopeBuilder {
    pub fn new() -> Self {
        Self {
            config: ScopeConfig::default(),
        }
    }

    pub fn max_tasks(mut self, max: usize) -> Self {
        self.config.max_tasks = max;
        self
    }

    pub fn propagate_panic(mut self, propagate: bool) -> Self {
        self.config.propagate_panic = propagate;
        self
    }

    pub fn await_all(mut self, await_all: bool) -> Self {
        self.config.await_all = await_all;
        self
    }

    pub fn build(self) -> ScopeConfig {
        self.config
    }
}

impl Default for ScopeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A scope for managing a group of tasks
pub struct TaskScope {
    config: ScopeConfig,
    active_count: AtomicUsize,
    completed_count: AtomicUsize,
    errors: Mutex<VecDeque<String>>,
}

impl TaskScope {
    /// Create a new task scope
    pub fn new(config: ScopeConfig) -> Self {
        Self {
            config,
            active_count: AtomicUsize::new(0),
            completed_count: AtomicUsize::new(0),
            errors: Mutex::new(VecDeque::new()),
        }
    }

    /// Create a new task scope with default configuration
    pub fn default_scope() -> Self {
        Self::new(ScopeConfig::default())
    }

    /// Get the number of active tasks
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Get the number of completed tasks
    pub fn completed_count(&self) -> usize {
        self.completed_count.load(Ordering::SeqCst)
    }

    /// Check if there are any active tasks
    pub fn has_active(&self) -> bool {
        self.active_count.load(Ordering::SeqCst) > 0
    }

    /// Record a task start
    pub fn task_started(&self) -> bool {
        let current = self.active_count.load(Ordering::SeqCst);
        if current >= self.config.max_tasks {
            return false;
        }
        self.active_count.store(current + 1, Ordering::SeqCst);
        true
    }

    /// Record a task completion
    pub fn task_completed(&self) {
        let current = self.active_count.load(Ordering::SeqCst);
        if current > 0 {
            self.active_count.store(current - 1, Ordering::SeqCst);
        }
        self.completed_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Record an error
    pub fn record_error(&self, error: String) {
        if let Ok(mut errors) = self.errors.lock() {
            errors.push_back(error);
        }
    }

    /// Get all recorded errors
    pub fn errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .map(|e| e.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        self.errors.lock().map(|e| !e.is_empty()).unwrap_or(false)
    }
}

/// A handle to a scoped task
pub struct ScopedTask<T> {
    scope: Arc<TaskScope>,
    result: Mutex<Option<T>>,
}

impl<T> ScopedTask<T> {
    /// Create a new scoped task
    pub fn new(scope: Arc<TaskScope>) -> Self {
        Self {
            scope,
            result: Mutex::new(None),
        }
    }

    /// Set the result of the task
    pub fn set_result(&self, value: T) {
        if let Ok(mut result) = self.result.lock() {
            *result = Some(value);
        }
    }

    /// Get the result of the task
    pub fn result(&self) -> Option<T>
    where
        T: Clone,
    {
        self.result.lock().ok().and_then(|r| r.clone())
    }

    /// Mark the task as completed
    pub fn complete(&self) {
        self.scope.task_completed();
    }
}

/// Extension trait for task scope
pub trait ScopeExt {
    fn spawn<F, T>(&self, f: F) -> Option<ScopedTask<T>>
    where
        F: FnOnce(&TaskScope) -> T;
}

impl ScopeExt for Arc<TaskScope> {
    fn spawn<F, T>(&self, f: F) -> Option<ScopedTask<T>>
    where
        F: FnOnce(&TaskScope) -> T,
    {
        if !self.task_started() {
            return None;
        }

        let scope = Arc::clone(self);
        let task = ScopedTask::new(Arc::clone(&scope));

        // In a real implementation, this would spawn a thread or async task
        // For now, we just run it synchronously
        let result = f(&scope);
        task.set_result(result);
        task.complete();

        Some(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_config_default() {
        let config = ScopeConfig::default();
        assert_eq!(config.max_tasks, 100);
        assert!(!config.propagate_panic);
        assert!(config.await_all);
    }

    #[test]
    fn test_scope_builder() {
        let config = ScopeBuilder::new()
            .max_tasks(50)
            .propagate_panic(true)
            .await_all(false)
            .build();

        assert_eq!(config.max_tasks, 50);
        assert!(config.propagate_panic);
        assert!(!config.await_all);
    }

    #[test]
    fn test_task_scope_basic() {
        let scope = TaskScope::default_scope();

        assert_eq!(scope.active_count(), 0);
        assert_eq!(scope.completed_count(), 0);
        assert!(!scope.has_active());

        // Start a task
        assert!(scope.task_started());
        assert_eq!(scope.active_count(), 1);
        assert!(scope.has_active());

        // Complete the task
        scope.task_completed();
        assert_eq!(scope.active_count(), 0);
        assert_eq!(scope.completed_count(), 1);
    }

    #[test]
    fn test_task_scope_max_tasks() {
        let config = ScopeConfig {
            max_tasks: 2,
            ..Default::default()
        };
        let scope = TaskScope::new(config);

        assert!(scope.task_started());
        assert!(scope.task_started());
        // Third task should fail
        assert!(!scope.task_started());
    }

    #[test]
    fn test_task_scope_errors() {
        let scope = TaskScope::default_scope();

        assert!(!scope.has_errors());
        assert!(scope.errors().is_empty());

        scope.record_error("Error 1".to_string());
        scope.record_error("Error 2".to_string());

        assert!(scope.has_errors());
        let errors = scope.errors();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_scoped_task() {
        let scope = Arc::new(TaskScope::default_scope());

        let task = scope.spawn(|_s| 42);

        assert!(task.is_some());
        let task = task.unwrap();

        // The spawn already calls task_completed
        assert!(scope.completed_count() >= 1);
        assert_eq!(task.result(), Some(42));
    }
}
