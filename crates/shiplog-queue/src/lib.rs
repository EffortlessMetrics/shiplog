//! Message queue abstractions for shiplog async task processing.
//!
//! This crate provides queue implementations for handling asynchronous
//! message processing within the shiplog pipeline.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

/// A queued message item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedItem<T> {
    /// Unique identifier for this item.
    pub id: String,
    /// The payload of the item.
    pub payload: T,
    /// Priority level (higher = more important).
    pub priority: i32,
    /// Number of retry attempts.
    pub retries: u32,
}

/// Message queue with FIFO and priority support.
pub struct MessageQueue<T> {
    items: Arc<RwLock<VecDeque<QueuedItem<T>>>>,
    max_size: usize,
}

impl<T: Clone> Default for MessageQueue<T> {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl<T: Clone> MessageQueue<T> {
    /// Create a new message queue with the specified maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            items: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
        }
    }

    /// Enqueue an item.
    pub fn enqueue(&self, payload: T, priority: i32) -> Result<QueuedItem<T>, QueueError> {
        let mut items = self.items.write().unwrap();

        if items.len() >= self.max_size {
            return Err(QueueError::QueueFull);
        }

        let item = QueuedItem {
            id: uuid_simple(),
            payload,
            priority,
            retries: 0,
        };

        // Insert in priority order (higher priority first)
        let pos = items
            .iter()
            .position(|i| i.priority < item.priority)
            .unwrap_or(items.len());

        items.insert(pos, item.clone());
        Ok(item)
    }

    /// Dequeue the next item.
    pub fn dequeue(&self) -> Option<QueuedItem<T>> {
        let mut items = self.items.write().unwrap();
        items.pop_front()
    }

    /// Peek at the next item without removing it.
    pub fn peek(&self) -> Option<QueuedItem<T>> {
        let items = self.items.read().unwrap();
        items.front().cloned()
    }

    /// Get the current queue length.
    pub fn len(&self) -> usize {
        let items = self.items.read().unwrap();
        items.len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        let items = self.items.read().unwrap();
        items.is_empty()
    }

    /// Clear all items from the queue.
    pub fn clear(&self) {
        let mut items = self.items.write().unwrap();
        items.clear();
    }

    /// Increment retry count for an item by ID.
    pub fn increment_retries(&self, id: &str) -> Option<u32> {
        let mut items = self.items.write().unwrap();
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.retries += 1;
            Some(item.retries)
        } else {
            None
        }
    }
}

/// Queue operation errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueueError {
    QueueFull,
    ItemNotFound,
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::QueueFull => write!(f, "Queue is at maximum capacity"),
            QueueError::ItemNotFound => write!(f, "Item not found in queue"),
        }
    }
}

impl std::error::Error for QueueError {}

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
    fn test_queue_enqueue_dequeue() {
        let queue: MessageQueue<String> = MessageQueue::new(10);

        queue.enqueue("item1".to_string(), 0).unwrap();
        queue.enqueue("item2".to_string(), 0).unwrap();

        assert_eq!(queue.len(), 2);

        let item = queue.dequeue().unwrap();
        assert_eq!(item.payload, "item1");

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_priority_ordering() {
        let queue: MessageQueue<String> = MessageQueue::new(10);

        queue.enqueue("low".to_string(), 1).unwrap();
        queue.enqueue("high".to_string(), 10).unwrap();
        queue.enqueue("medium".to_string(), 5).unwrap();

        let item = queue.dequeue().unwrap();
        assert_eq!(item.payload, "high");

        let item = queue.dequeue().unwrap();
        assert_eq!(item.payload, "medium");

        let item = queue.dequeue().unwrap();
        assert_eq!(item.payload, "low");
    }

    #[test]
    fn test_queue_max_size() {
        let queue: MessageQueue<String> = MessageQueue::new(2);

        queue.enqueue("1".to_string(), 0).unwrap();
        queue.enqueue("2".to_string(), 0).unwrap();

        let result = queue.enqueue("3".to_string(), 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), QueueError::QueueFull);
    }

    #[test]
    fn test_queue_peek() {
        let queue: MessageQueue<String> = MessageQueue::new(10);

        queue.enqueue("first".to_string(), 0).unwrap();
        queue.enqueue("second".to_string(), 0).unwrap();

        let peeked = queue.peek().unwrap();
        assert_eq!(peeked.payload, "first");

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_queue_clear() {
        let queue: MessageQueue<String> = MessageQueue::new(10);

        queue.enqueue("item".to_string(), 0).unwrap();
        assert!(!queue.is_empty());

        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_retries() {
        let queue: MessageQueue<String> = MessageQueue::new(10);

        let item = queue.enqueue("test".to_string(), 0).unwrap();
        let retries = queue.increment_retries(&item.id);

        assert_eq!(retries, Some(1));
    }

    #[test]
    fn test_queue_error_display() {
        assert_eq!(
            QueueError::QueueFull.to_string(),
            "Queue is at maximum capacity"
        );
        assert_eq!(
            QueueError::ItemNotFound.to_string(),
            "Item not found in queue"
        );
    }
}
