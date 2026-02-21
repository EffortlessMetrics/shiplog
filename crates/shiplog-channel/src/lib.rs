//! Channel utilities for shiplog.
//!
//! This crate provides channel utilities for communication between async tasks.

use tokio::sync::{mpsc, broadcast};

/// Configuration for channel operations
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub buffer_size: usize,
    pub channel_type: ChannelType,
    pub name: String,
}

/// Type of channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Mpsc,
    Broadcast,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            channel_type: ChannelType::Mpsc,
            name: "default".to_string(),
        }
    }
}

/// Builder for creating channels
#[derive(Debug)]
pub struct ChannelBuilder {
    config: ChannelConfig,
}

impl ChannelBuilder {
    pub fn new() -> Self {
        Self {
            config: ChannelConfig::default(),
        }
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn channel_type(mut self, channel_type: ChannelType) -> Self {
        self.config.channel_type = channel_type;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    pub fn build(self) -> ChannelConfig {
        self.config
    }
}

impl Default for ChannelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an mpsc channel with configuration
pub fn create_mpsc_channel<T>(buffer_size: usize) -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    mpsc::channel(buffer_size)
}

/// Create a broadcast channel with configuration
pub fn create_broadcast_channel<T: Clone>(buffer_size: usize) -> (broadcast::Sender<T>, broadcast::Receiver<T>) {
    broadcast::channel(buffer_size)
}

/// Message wrapper for channel messages
#[derive(Debug, Clone)]
pub struct ChannelMessage<T> {
    pub payload: T,
    pub metadata: ChannelMetadata,
}

/// Metadata for channel messages
#[derive(Debug, Clone, Default)]
pub struct ChannelMetadata {
    pub sender_id: Option<String>,
    pub timestamp_ms: u64,
    pub priority: u8,
}

impl<T> ChannelMessage<T> {
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            metadata: ChannelMetadata::default(),
        }
    }

    pub fn with_sender(mut self, sender_id: &str) -> Self {
        self.metadata.sender_id = Some(sender_id.to_string());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.metadata.priority = priority;
        self
    }
}

/// Sender wrapper for mpsc channels
pub struct MpscSender<T> {
    sender: mpsc::Sender<T>,
}

impl<T> MpscSender<T> {
    pub fn new(sender: mpsc::Sender<T>) -> Self {
        Self { sender }
    }

    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(value).await
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    pub fn capacity(&self) -> usize {
        self.sender.capacity()
    }
}

/// Receiver wrapper for mpsc channels
pub struct MpscReceiver<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> MpscReceiver<T> {
    pub fn new(receiver: mpsc::Receiver<T>) -> Self {
        Self { receiver }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

/// Broadcast sender wrapper
pub struct BroadcastSender<T: Clone> {
    sender: broadcast::Sender<T>,
}

impl<T: Clone> BroadcastSender<T> {
    pub fn new(sender: broadcast::Sender<T>) -> Self {
        Self { sender }
    }

    pub fn send(&self, value: T) -> Result<usize, broadcast::error::SendError<T>> {
        self.sender.send(value)
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Broadcast receiver wrapper
pub struct BroadcastReceiver<T: Clone> {
    receiver: broadcast::Receiver<T>,
}

impl<T: Clone> BroadcastReceiver<T> {
    pub fn new(receiver: broadcast::Receiver<T>) -> Self {
        Self { receiver }
    }

    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        self.receiver.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_config_default() {
        let config = ChannelConfig::default();
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.channel_type, ChannelType::Mpsc);
        assert_eq!(config.name, "default");
    }

    #[test]
    fn test_channel_builder() {
        let config = ChannelBuilder::new()
            .buffer_size(200)
            .channel_type(ChannelType::Broadcast)
            .name("test-channel")
            .build();
        
        assert_eq!(config.buffer_size, 200);
        assert_eq!(config.channel_type, ChannelType::Broadcast);
        assert_eq!(config.name, "test-channel");
    }

    #[test]
    fn test_channel_message() {
        let msg = ChannelMessage::new(42)
            .with_sender("sender-1")
            .with_priority(5);
        
        assert_eq!(msg.payload, 42);
        assert_eq!(msg.metadata.sender_id, Some("sender-1".to_string()));
        assert_eq!(msg.metadata.priority, 5);
    }

    #[tokio::test]
    async fn test_mpsc_channel() {
        let (tx, rx) = create_mpsc_channel::<i32>(10);
        let sender = MpscSender::new(tx);
        let mut receiver = MpscReceiver::new(rx);
        
        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        
        assert_eq!(receiver.recv().await, Some(1));
        assert_eq!(receiver.recv().await, Some(2));
    }

    #[tokio::test]
    async fn test_broadcast_channel() {
        let (tx, rx) = create_broadcast_channel::<i32>(10);
        let sender = BroadcastSender::new(tx);
        let mut receiver = BroadcastReceiver::new(rx);
        
        sender.send(42).unwrap();
        
        let result = receiver.recv().await.unwrap();
        assert_eq!(result, 42);
    }
}
