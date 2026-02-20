//! Pub/sub messaging utilities for shiplog event distribution.
//!
//! This crate provides a simple pub/sub pattern implementation for distributing
//! events to multiple subscribers within the shiplog system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A message published through the pub/sub system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<T> {
    /// The topic this message was published to.
    pub topic: String,
    /// The payload of the message.
    pub payload: T,
    /// Unique identifier for this message.
    pub id: String,
}

/// A subscriber callback type.
pub type Subscriber<T> = Box<dyn Fn(Message<T>) + Send + Sync>;

/// Pub/Sub broker for distributing messages to subscribers.
pub struct PubSub<T> {
    subscribers: Arc<RwLock<HashMap<String, Vec<Subscriber<T>>>>>,
}

impl<T: Clone + Send + 'static> Default for PubSub<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + 'static> PubSub<T> {
    /// Create a new PubSub broker.
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to a topic.
    pub fn subscribe<F>(&self, topic: &str, callback: F)
    where
        F: Fn(Message<T>) + Send + Sync + 'static,
    {
        let mut subs = self.subscribers.write().unwrap();
        subs.entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
    }

    /// Publish a message to a topic.
    pub fn publish(&self, topic: &str, payload: T) -> Message<T> {
        let message = Message {
            topic: topic.to_string(),
            payload,
            id: uuid_simple(),
        };

        let subs = self.subscribers.read().unwrap();
        if let Some(topic_subs) = subs.get(topic) {
            for sub in topic_subs {
                sub(message.clone());
            }
        }

        message
    }

    /// Get the number of subscribers for a topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        let subs = self.subscribers.read().unwrap();
        subs.get(topic).map(|s| s.len()).unwrap_or(0)
    }

    /// Get the list of topics with subscribers.
    pub fn topics(&self) -> Vec<String> {
        let subs = self.subscribers.read().unwrap();
        subs.keys().cloned().collect()
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
    fn test_pubsub_subscribe_and_publish() {
        let pubsub: PubSub<String> = PubSub::new();
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        pubsub.subscribe("test-topic", move |msg| {
            received_clone.write().unwrap().push(msg.payload);
        });

        pubsub.publish("test-topic", "hello".to_string());
        pubsub.publish("test-topic", "world".to_string());

        let msgs = received.read().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0], "hello");
        assert_eq!(msgs[1], "world");
    }

    #[test]
    fn test_pubsub_no_subscribers() {
        let pubsub: PubSub<String> = PubSub::new();
        let result = pubsub.publish("empty-topic", "test".to_string());
        assert_eq!(result.topic, "empty-topic");
    }

    #[test]
    fn test_subscriber_count() {
        let pubsub: PubSub<String> = PubSub::new();
        
        assert_eq!(pubsub.subscriber_count("test"), 0);
        
        pubsub.subscribe("test", |_: Message<String>| {});
        assert_eq!(pubsub.subscriber_count("test"), 1);
        
        pubsub.subscribe("test", |_: Message<String>| {});
        assert_eq!(pubsub.subscriber_count("test"), 2);
    }

    #[test]
    fn test_topics() {
        let pubsub: PubSub<String> = PubSub::new();
        
        pubsub.subscribe("topic-a", |_: Message<String>| {});
        pubsub.subscribe("topic-b", |_: Message<String>| {});
        
        let topics = pubsub.topics();
        assert!(topics.contains(&"topic-a".to_string()));
        assert!(topics.contains(&"topic-b".to_string()));
    }

    #[test]
    fn test_message_id_unique() {
        let pubsub: PubSub<String> = PubSub::new();
        
        let msg1 = pubsub.publish("test", "a".to_string());
        let msg2 = pubsub.publish("test", "b".to_string());
        
        assert_ne!(msg1.id, msg2.id);
    }
}
