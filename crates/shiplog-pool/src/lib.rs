//! Object pooling utilities for shiplog.
//!
//! This crate provides object pooling utilities for efficient resource reuse.

use std::collections::VecDeque;
use std::sync::Mutex;

/// Configuration for object pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: usize,
    pub min_size: usize,
    pub preallocate: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_size: 2,
            preallocate: false,
        }
    }
}

/// Builder for pool configuration
#[derive(Debug)]
pub struct PoolBuilder {
    config: PoolConfig,
}

impl PoolBuilder {
    pub fn new() -> Self {
        Self {
            config: PoolConfig::default(),
        }
    }

    pub fn max_size(mut self, size: usize) -> Self {
        self.config.max_size = size;
        self
    }

    pub fn min_size(mut self, size: usize) -> Self {
        self.config.min_size = size;
        self
    }

    pub fn preallocate(mut self, preallocate: bool) -> Self {
        self.config.preallocate = preallocate;
        self
    }

    pub fn build(self) -> PoolConfig {
        self.config
    }
}

impl Default for PoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple object pool for reusing allocated objects
pub struct ObjectPool<T> {
    pool: Mutex<VecDeque<T>>,
    max_size: usize,
}

impl<T> ObjectPool<T> {
    /// Create a new pool with the given configuration
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    /// Get an object from the pool, or create a new one if empty
    pub fn get(&self) -> Option<T>
    where
        T: Default,
    {
        let mut pool = self.pool.lock().ok()?;
        pool.pop_front()
    }

    /// Return an object to the pool
    pub fn put(&self, item: T) -> bool {
        if let Ok(mut pool) = self.pool.lock()
            && pool.len() < self.max_size
        {
            pool.push_back(item);
            return true;
        }
        false
    }

    /// Get the current number of objects in the pool
    pub fn len(&self) -> usize {
        self.pool.lock().map(|p| p.len()).unwrap_or(0)
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.pool.lock().map(|p| p.is_empty()).unwrap_or(true)
    }
}

/// Pooled wrapper that automatically returns the object to the pool
pub struct Pooled<T: Default + 'static> {
    pool: Option<&'static ObjectPool<T>>,
    value: T,
}

impl<T: Default + 'static> Pooled<T> {
    /// Create a new pooled object
    pub fn new(pool: &'static ObjectPool<T>, value: T) -> Self {
        Self {
            pool: Some(pool),
            value,
        }
    }

    /// Get a reference to the underlying value
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference to the underlying value
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Default + 'static> Drop for Pooled<T> {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            let _ = pool.put(std::mem::take(&mut self.value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_size, 10);
        assert_eq!(config.min_size, 2);
        assert!(!config.preallocate);
    }

    #[test]
    fn test_pool_builder() {
        let config = PoolBuilder::new()
            .max_size(20)
            .min_size(5)
            .preallocate(true)
            .build();

        assert_eq!(config.max_size, 20);
        assert_eq!(config.min_size, 5);
        assert!(config.preallocate);
    }

    #[test]
    fn test_object_pool_basic() {
        let pool: ObjectPool<String> = ObjectPool::new(5);

        // Pool should start empty
        assert!(pool.is_empty());

        // Put some items
        assert!(pool.put(String::from("item1")));
        assert!(pool.put(String::from("item2")));
        assert_eq!(pool.len(), 2);

        // Get an item
        let item = pool.get();
        assert!(item.is_some());
        assert_eq!(pool.len(), 1);

        // Return the item
        if let Some(s) = item {
            assert!(pool.put(s));
        }
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_pool_max_size() {
        let pool: ObjectPool<i32> = ObjectPool::new(2);

        assert!(pool.put(1));
        assert!(pool.put(2));
        // Pool is full, should not accept more
        assert!(!pool.put(3));
    }

    #[test]
    fn test_pooled_drops() {
        use std::sync::LazyLock;
        static POOL: LazyLock<ObjectPool<String>> = LazyLock::new(|| ObjectPool::new(5));

        {
            let _pooled = Pooled::new(&POOL, String::from("test"));
            assert_eq!(POOL.len(), 0);
        }
        // After drop, item should be returned to pool
        assert_eq!(POOL.len(), 1);
    }
}
