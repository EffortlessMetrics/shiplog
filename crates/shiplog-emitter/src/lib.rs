//! Event emitter pattern utilities for shiplog.
//!
//! This crate provides event emitter implementations for publish-subscribe patterns.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A simple event emitter that allows subscribing to and emitting events.
pub struct Emitter<T> {
    listeners: HashMap<String, Vec<Box<dyn FnMut(&T) + Send + Sync>>>,
}

impl<T> Emitter<T> {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    /// Subscribe to an event with a callback
    pub fn on<F>(&mut self, event: &str, callback: F)
    where
        F: FnMut(&T) + Send + Sync + 'static,
    {
        self.listeners
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
    }

    /// Emit an event to all subscribers
    pub fn emit(&mut self, event: &str, data: &T) {
        if let Some(listeners) = self.listeners.get_mut(event) {
            for listener in listeners {
                listener(data);
            }
        }
    }

    /// Remove all listeners for an event
    pub fn off(&mut self, event: &str) {
        self.listeners.remove(event);
    }

    /// Check if there are any listeners for an event
    pub fn has_listeners(&self, event: &str) -> bool {
        self.listeners
            .get(event)
            .map(|l| !l.is_empty())
            .unwrap_or(false)
    }
}

impl<T> Default for Emitter<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe version of the emitter using Arc and RwLock
pub type SharedEmitter<T> = Arc<RwLock<Emitter<T>>>;

/// Create a new shared emitter
pub fn shared<T>() -> SharedEmitter<T> {
    Arc::new(RwLock::new(Emitter::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_emitter_basic() {
        let mut emitter: Emitter<String> = Emitter::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        {
            let received = received.clone();
            emitter.on("test", move |data| {
                received.lock().unwrap().push(data.clone());
            });
        }

        emitter.emit("test", &"hello".to_string());
        emitter.emit("test", &"world".to_string());

        assert_eq!(*received.lock().unwrap(), vec!["hello", "world"]);
    }

    #[test]
    fn test_emitter_multiple_events() {
        let mut emitter: Emitter<i32> = Emitter::new();
        let event_a = Arc::new(Mutex::new(Vec::new()));
        let event_b = Arc::new(Mutex::new(Vec::new()));

        {
            let event_a = event_a.clone();
            emitter.on("a", move |v| event_a.lock().unwrap().push(*v));
        }
        {
            let event_b = event_b.clone();
            emitter.on("b", move |v| event_b.lock().unwrap().push(*v));
        }

        emitter.emit("a", &1);
        emitter.emit("b", &2);
        emitter.emit("a", &3);

        assert_eq!(*event_a.lock().unwrap(), vec![1, 3]);
        assert_eq!(*event_b.lock().unwrap(), vec![2]);
    }

    #[test]
    fn test_emitter_off() {
        let mut emitter: Emitter<String> = Emitter::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        {
            let received = received.clone();
            emitter.on("test", move |data| {
                received.lock().unwrap().push(data.clone());
            });
        }

        emitter.emit("test", &"before".to_string());
        emitter.off("test");
        emitter.emit("test", &"after".to_string());

        assert_eq!(*received.lock().unwrap(), vec!["before"]);
    }

    #[test]
    fn test_emitter_has_listeners() {
        let mut emitter: Emitter<String> = Emitter::new();

        assert!(!emitter.has_listeners("test"));

        emitter.on("test", |_v| {});
        assert!(emitter.has_listeners("test"));

        emitter.off("test");
        assert!(!emitter.has_listeners("test"));
    }

    #[test]
    fn test_shared_emitter() {
        let emitter: SharedEmitter<String> = shared();
        
        {
            let mut e = emitter.write().unwrap();
            e.on("test", |data| {
                assert_eq!(data, "hello");
            });
        }

        emitter.write().unwrap().emit("test", &"hello".to_string());
    }
}
