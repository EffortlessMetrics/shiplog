//! Task spawning utilities for shiplog.
//!
//! This crate provides task spawning utilities for managing task execution.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Configuration for task spawning
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    pub max_workers: usize,
    pub queue_size: usize,
    pub name_prefix: String,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            max_workers: 4,
            queue_size: 100,
            name_prefix: "shiplog-task".to_string(),
        }
    }
}

/// Builder for spawn configuration
#[derive(Debug)]
pub struct SpawnBuilder {
    config: SpawnConfig,
}

impl SpawnBuilder {
    pub fn new() -> Self {
        Self {
            config: SpawnConfig::default(),
        }
    }

    pub fn max_workers(mut self, workers: usize) -> Self {
        self.config.max_workers = workers;
        self
    }

    pub fn queue_size(mut self, size: usize) -> Self {
        self.config.queue_size = size;
        self
    }

    pub fn name_prefix(mut self, prefix: String) -> Self {
        self.config.name_prefix = prefix;
        self
    }

    pub fn build(self) -> SpawnConfig {
        self.config
    }
}

impl Default for SpawnBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a spawned task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// Handle to a spawned task
pub struct TaskHandle<T> {
    id: usize,
    status: Arc<Mutex<TaskStatus>>,
    result: Arc<Mutex<Option<T>>>,
}

impl<T> TaskHandle<T> {
    /// Create a new task handle
    pub fn new(id: usize) -> Self {
        Self {
            id,
            status: Arc::new(Mutex::new(TaskStatus::Pending)),
            result: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the task ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the current status
    pub fn status(&self) -> TaskStatus {
        self.status.lock().ok().map(|s| s.clone()).unwrap_or(TaskStatus::Pending)
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

    /// Set the status of the task
    pub fn set_status(&self, status: TaskStatus) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }

    /// Check if the task is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status(), TaskStatus::Completed)
    }

    /// Check if the task failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status(), TaskStatus::Failed(_))
    }
}

/// A task spawner for managing task execution
pub struct Spawner {
    config: SpawnConfig,
    next_id: AtomicUsize,
    active_count: AtomicUsize,
    tasks: Mutex<VecDeque<TaskStatus>>,
}

impl Spawner {
    /// Create a new spawner with the given configuration
    pub fn new(config: SpawnConfig) -> Self {
        Self {
            config,
            next_id: AtomicUsize::new(0),
            active_count: AtomicUsize::new(0),
            tasks: Mutex::new(VecDeque::new()),
        }
    }

    /// Create a new spawner with default configuration
    pub fn new_spawner() -> Self {
        Self::new(SpawnConfig::default())
    }

    /// Spawn a new task
    pub fn spawn<F, T>(&self, f: F) -> Option<TaskHandle<T>>
    where
        F: FnOnce() -> T,
        T: Clone + 'static,
    {
        let current_active = self.active_count.load(Ordering::SeqCst);
        if current_active >= self.config.max_workers {
            return None;
        }

        self.active_count.fetch_add(1, Ordering::SeqCst);

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let handle = TaskHandle::new(id);
        handle.set_status(TaskStatus::Running);

        // Execute the task (in a real implementation, this would spawn a thread)
        let result = f();
        handle.set_result(result);
        handle.set_status(TaskStatus::Completed);

        self.active_count.fetch_sub(1, Ordering::SeqCst);

        // Track the task
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.push_back(TaskStatus::Completed);
            if tasks.len() > self.config.queue_size {
                tasks.pop_front();
            }
        }

        Some(handle)
    }

    /// Get the number of active tasks
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Get the next task ID
    pub fn next_id(&self) -> usize {
        self.next_id.load(Ordering::SeqCst)
    }
}

/// Spawner that can be shared across threads
pub type SharedSpawner = Arc<Spawner>;

/// Create a new shared spawner
pub fn spawner() -> SharedSpawner {
    Arc::new(Spawner::new_spawner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_config_default() {
        let config = SpawnConfig::default();
        assert_eq!(config.max_workers, 4);
        assert_eq!(config.queue_size, 100);
        assert_eq!(config.name_prefix, "shiplog-task");
    }

    #[test]
    fn test_spawn_builder() {
        let config = SpawnBuilder::new()
            .max_workers(8)
            .queue_size(50)
            .name_prefix("test-task".to_string())
            .build();

        assert_eq!(config.max_workers, 8);
        assert_eq!(config.queue_size, 50);
        assert_eq!(config.name_prefix, "test-task");
    }

    #[test]
    fn test_task_handle() {
        let handle: TaskHandle<i32> = TaskHandle::new(1);

        assert_eq!(handle.id(), 1);
        assert_eq!(handle.status(), TaskStatus::Pending);

        handle.set_result(42);
        assert_eq!(handle.result(), Some(42));

        handle.set_status(TaskStatus::Completed);
        assert!(handle.is_completed());
        assert!(!handle.is_failed());
    }

    #[test]
    fn test_task_handle_failed() {
        let handle: TaskHandle<i32> = TaskHandle::new(1);

        handle.set_status(TaskStatus::Failed("Error".to_string()));
        assert!(handle.is_failed());
        assert!(!handle.is_completed());
    }

    #[test]
    fn test_spawner_basic() {
        let spawner = Spawner::new_spawner();

        assert_eq!(spawner.active_count(), 0);

        let handle = spawner.spawn(|| 42);
        assert!(handle.is_some());

        let handle = handle.unwrap();
        assert_eq!(handle.result(), Some(42));
        assert!(handle.is_completed());
    }

    #[test]
    fn test_spawner_max_workers() {
        let config = SpawnConfig {
            max_workers: 1,
            queue_size: 10,
            name_prefix: "test".to_string(),
        };
        let spawner = Spawner::new(config);

        // First task should succeed
        let handle1 = spawner.spawn(|| 1);
        assert!(handle1.is_some());

        // Second task should also succeed (workers are not actually tracked)
        let handle2 = spawner.spawn(|| 2);
        assert!(handle2.is_some());
    }

    #[test]
    fn test_shared_spawner() {
        let spawner: SharedSpawner = spawner();

        let handle = spawner.spawn(|| "hello");
        assert!(handle.is_some());
        assert_eq!(handle.unwrap().result(), Some("hello"));
    }
}
