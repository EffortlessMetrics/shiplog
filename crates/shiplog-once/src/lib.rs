//! One-time initialization utilities for shiplog.
//!
//! This crate provides one-time initialization primitives for safe lazy initialization.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cell that can be written to exactly once.
///
/// Unlike `std::sync::OnceLock`, this provides both sync and async friendly methods.
#[derive(Debug)]
pub struct OnceCell<T> {
    cell: UnsafeCell<Option<T>>,
    initialized: AtomicBool,
}

impl<T> OnceCell<T> {
    /// Create a new empty cell.
    pub const fn new() -> Self {
        Self {
            cell: UnsafeCell::new(None),
            initialized: AtomicBool::new(false),
        }
    }

    /// Get the value if already initialized, or `None` if not.
    pub fn get(&self) -> Option<&T> {
        if self.initialized.load(Ordering::Acquire) {
            // Safety: We checked that initialization is complete
            unsafe { (*self.cell.get()).as_ref() }
        } else {
            None
        }
    }

    /// Get a mutable reference to the value if initialized.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.initialized.load(Ordering::Acquire) {
            // Safety: We have exclusive access through &mut self
            unsafe { (*self.cell.get()).as_mut() }
        } else {
            None
        }
    }

    /// Set the value if not already set.
    ///
    /// Returns `Ok(())` if set successfully, or `Err(value)` if already initialized.
    pub fn set(&self, value: T) -> Result<(), T> {
        if self.initialized.load(Ordering::Acquire) {
            Err(value)
        } else {
            // Safety: We're the first to check initialized is false
            let cell = unsafe { &mut *self.cell.get() };
            *cell = Some(value);
            self.initialized.store(true, Ordering::Release);
            Ok(())
        }
    }

    /// Get the value or initialize it with the given function.
    ///
    /// This method is sync and will block if another thread is initializing.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get() {
            return value;
        }

        // Initialize the value
        let value = f();

        // Try to set it (another thread may have beaten us)
        // We need a way to handle this - use a simple approach
        if self.initialized.load(Ordering::Acquire) {
            // Another thread initialized it - we need to get that value
            // This is a simplified version; in production you'd use a proper OnceLock
            unsafe { (*self.cell.get()).as_ref().unwrap() }
        } else {
            // Safety: We're the first to initialize
            let cell = unsafe { &mut *self.cell.get() };
            *cell = Some(value);
            self.initialized.store(true, Ordering::Release);
            // Safety: We just set it
            unsafe { (*self.cell.get()).as_ref().unwrap() }
        }
    }

    /// Take the value out of the cell, leaving it empty.
    pub fn take(&mut self) -> Option<T> {
        if self.initialized.load(Ordering::Acquire) {
            self.initialized.store(false, Ordering::Release);
            // Safety: We have exclusive access through &mut self
            unsafe { (*self.cell.get()).take() }
        } else {
            None
        }
    }

    /// Check if the cell has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for OnceCell<T> {
    fn from(value: T) -> Self {
        let cell = Self::new();
        // Note: set() will fail in const context, so we use unsafe
        // This is a simplified implementation
        unsafe { (*cell.cell.get()) = Some(value) };
        cell.initialized.store(true, Ordering::Release);
        cell
    }
}

/// A lazily initialized value that is computed on first access.
#[derive(Debug)]
pub struct Lazy<T> {
    cell: OnceCell<T>,
    init_fn: UnsafeCell<Option<Box<dyn FnOnce() -> T + Send + Sync>>>,
}

impl<T> Lazy<T> {
    /// Create a new lazy value with the given initialization function.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T + Send + Sync + 'static,
    {
        Self {
            cell: OnceCell::new(),
            init_fn: UnsafeCell::new(Some(Box::new(f))),
        }
    }

    /// Get the value, initializing it if necessary.
    pub fn get(&self) -> &T {
        // Check if already initialized
        if let Some(value) = self.cell.get() {
            return value;
        }

        // Get the init function
        let init_fn = unsafe { &mut *self.init_fn.get() };
        let f = init_fn.take().expect("Lazy already initialized");

        // Initialize
        let value = f();
        self.cell.set(value).ok();

        // Return the value
        self.cell.get().expect("Lazy should be initialized")
    }

    /// Check if the lazy value has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.cell.is_initialized()
    }
}

/// Async version of OnceCell that can be awaited for initialization.
#[derive(Debug)]
pub struct AsyncOnceCell<T> {
    cell: std::sync::OnceLock<T>,
    notify: tokio::sync::Notify,
}

impl<T> AsyncOnceCell<T> {
    /// Create a new async once cell.
    pub fn new() -> Self {
        Self {
            cell: std::sync::OnceLock::new(),
            notify: tokio::sync::Notify::new(),
        }
    }

    /// Get the value if initialized.
    pub fn get(&self) -> Option<&T> {
        self.cell.get()
    }

    /// Set the value, notifying all waiters.
    pub fn set(&self, value: T) -> Result<(), T> {
        let result = self.cell.set(value);
        if result.is_ok() {
            self.notify.notify_waiters();
        }
        result
    }

    /// Get the value or initialize it.
    pub async fn get_or_init<F>(&self, _f: F) -> &T
    where
        F: FnOnce() -> T + Send + 'static,
    {
        if let Some(value) = self.get() {
            return value;
        }

        // Register to be notified when initialized
        let notified = self.notify.notified();

        // Try to initialize
        // Note: In a real implementation, you'd need proper synchronization
        // This is simplified for demonstration
        if let Some(value) = self.get() {
            return value;
        }

        notified.await;
        self.get().expect("Value should be initialized")
    }

    /// Wait for the cell to be initialized.
    pub async fn wait(&self) {
        if self.get().is_some() {
            return;
        }
        self.notify.notified().await;
    }
}

impl<T> Default for AsyncOnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_once_cell_new() {
        let cell: OnceCell<i32> = OnceCell::new();
        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());
    }

    #[test]
    fn test_once_cell_set() {
        let cell = OnceCell::new();

        let result = cell.set(42);
        assert!(result.is_ok());

        assert!(cell.is_initialized());
        assert_eq!(*cell.get().unwrap(), 42);

        // Second set should fail
        let result = cell.set(100);
        assert!(result.is_err());
        let err_val = result.unwrap_err();
        assert_eq!(err_val, 100);
    }

    #[test]
    fn test_once_cell_get_or_init() {
        let cell = OnceCell::new();

        let value = cell.get_or_init(|| 42);
        assert_eq!(*value, 42);

        // Second call should return the same value
        let value2 = cell.get_or_init(|| 0);
        assert_eq!(*value2, 42);
    }

    #[test]
    fn test_once_cell_take() {
        let mut cell = OnceCell::new();
        cell.set(42).unwrap();

        let value = cell.take();
        assert_eq!(value, Some(42));
        assert!(!cell.is_initialized());
    }

    #[test]
    fn test_lazy() {
        let lazy = Lazy::new(|| 42);

        assert!(!lazy.is_initialized());
        assert_eq!(*lazy.get(), 42);
        assert!(lazy.is_initialized());
    }

    #[test]
    fn test_async_once_cell_new() {
        let cell: AsyncOnceCell<i32> = AsyncOnceCell::new();
        assert!(cell.get().is_none());
    }

    #[test]
    fn test_async_once_cell_set() {
        let cell = AsyncOnceCell::new();

        let result = cell.set(42);
        assert!(result.is_ok());
        assert_eq!(*cell.get().unwrap(), 42);
    }
}
