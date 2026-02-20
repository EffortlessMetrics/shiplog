//! Event bus for internal messaging within shiplog.
//!
//! This crate provides an event bus implementation for decoupled internal
//! communication between shiplog components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// An event in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier.
    pub id: String,
    /// Event type/name.
    pub event_type: String,
    /// Event payload as JSON string.
    pub payload: String,
    /// When the event was created.
    pub timestamp: DateTime<Utc>,
    /// Source component that emitted the event.
    pub source: String,
}

impl Event {
    /// Create a new event.
    pub fn new(event_type: &str, payload: String, source: &str) -> Self {
        Self {
            id: uuid_simple(),
            event_type: event_type.to_string(),
            payload,
            timestamp: Utc::now(),
            source: source.to_string(),
        }
    }

    /// Create a new event with JSON-serializable payload.
    pub fn with_payload<T: Serialize>(event_type: &str, payload: &T, source: &str) -> Self {
        let payload_str = serde_json::to_string(payload).unwrap_or_default();
        Self::new(event_type, payload_str, source)
    }
}

/// Event handler callback.
pub type EventHandler = Box<dyn Fn(Event) + Send + Sync>;

/// The event bus for distributing events to handlers.
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<String, Vec<EventHandler>>>>,
    event_history: Arc<RwLock<Vec<Event>>>,
    max_history: usize,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(100)
    }
}

impl EventBus {
    /// Create a new event bus.
    pub fn new(max_history: usize) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    /// Register a handler for an event type.
    pub fn subscribe<F>(&self, event_type: &str, handler: F)
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        handlers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(handler));
    }

    /// Emit an event to all registered handlers.
    pub fn emit(&self, event: Event) {
        // Store in history
        {
            let mut history = self.event_history.write().unwrap();
            if history.len() >= self.max_history {
                history.remove(0);
            }
            history.push(event.clone());
        }

        // Notify handlers
        let handlers = self.handlers.read().unwrap();
        if let Some(type_handlers) = handlers.get(&event.event_type) {
            for handler in type_handlers {
                handler(event.clone());
            }
        }
    }

    /// Emit a simple event.
    pub fn emit_simple(&self, event_type: &str, payload: String, source: &str) {
        let event = Event::new(event_type, payload, source);
        self.emit(event);
    }

    /// Get event history.
    pub fn history(&self) -> Vec<Event> {
        let history = self.event_history.read().unwrap();
        history.clone()
    }

    /// Get events of a specific type from history.
    pub fn history_by_type(&self, event_type: &str) -> Vec<Event> {
        let history = self.event_history.read().unwrap();
        history
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Clear event history.
    pub fn clear_history(&self) {
        let mut history = self.event_history.write().unwrap();
        history.clear();
    }

    /// Get the number of handlers for an event type.
    pub fn handler_count(&self, event_type: &str) -> usize {
        let handlers = self.handlers.read().unwrap();
        handlers
            .get(event_type)
            .map(|h| h.len())
            .unwrap_or(0)
    }

    /// Get all registered event types.
    pub fn event_types(&self) -> Vec<String> {
        let handlers = self.handlers.read().unwrap();
        handlers.keys().cloned().collect()
    }
}

/// Generate a simple UUID-like identifier.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}-{:x}", duration.as_secs(), duration.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("test.event", "payload".to_string(), "source");
        
        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.payload, "payload");
        assert_eq!(event.source, "source");
        assert!(!event.id.is_empty());
    }

    #[test]
    fn test_event_with_payload() {
        #[derive(Serialize)]
        struct Payload {
            value: i32,
        }
        
        let event = Event::with_payload("test", &Payload { value: 42 }, "source");
        
        assert_eq!(event.event_type, "test");
        assert!(event.payload.contains("42"));
    }

    #[test]
    fn test_eventbus_subscribe_emit() {
        let bus = EventBus::new(100);
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        bus.subscribe("user.created", move |event| {
            received_clone.write().unwrap().push(event);
        });

        bus.emit_simple("user.created", r#"{"name":"test"}"#.to_string(), "test");

        let events = received.read().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "user.created");
    }

    #[test]
    fn test_eventbus_history() {
        let bus = EventBus::new(10);
        
        bus.emit_simple("event.1", "data1".to_string(), "source");
        bus.emit_simple("event.2", "data2".to_string(), "source");
        
        let history = bus.history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_eventbus_history_by_type() {
        let bus = EventBus::new(10);
        
        bus.emit_simple("same.event", "a".to_string(), "source");
        bus.emit_simple("other.event", "b".to_string(), "source");
        bus.emit_simple("same.event", "c".to_string(), "source");
        
        let same_events = bus.history_by_type("same.event");
        assert_eq!(same_events.len(), 2);
    }

    #[test]
    fn test_eventbus_clear_history() {
        let bus = EventBus::new(10);
        
        bus.emit_simple("event", "data".to_string(), "source");
        assert!(!bus.history().is_empty());
        
        bus.clear_history();
        assert!(bus.history().is_empty());
    }

    #[test]
    fn test_eventbus_handler_count() {
        let bus = EventBus::new(10);
        
        assert_eq!(bus.handler_count("test"), 0);
        
        bus.subscribe("test", |_: Event| {});
        assert_eq!(bus.handler_count("test"), 1);
        
        bus.subscribe("test", |_: Event| {});
        assert_eq!(bus.handler_count("test"), 2);
    }

    #[test]
    fn test_eventbus_event_types() {
        let bus = EventBus::new(10);
        
        bus.subscribe("type.a", |_: Event| {});
        bus.subscribe("type.b", |_: Event| {});
        
        let types = bus.event_types();
        assert!(types.contains(&"type.a".to_string()));
        assert!(types.contains(&"type.b".to_string()));
    }

    #[test]
    fn test_eventbus_history_limit() {
        let bus = EventBus::new(3);
        
        for i in 0..5 {
            bus.emit_simple("event", format!("data{}", i), "source");
        }
        
        assert_eq!(bus.history().len(), 3);
    }
}
