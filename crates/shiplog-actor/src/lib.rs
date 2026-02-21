//! Actor model utilities for shiplog.
//!
//! This crate provides basic actor model utilities for asynchronous message handling.

use tokio::sync::mpsc;
use std::fmt;

/// Configuration for actor behavior
#[derive(Debug, Clone)]
pub struct ActorConfig {
    pub name: String,
    pub buffer_size: usize,
    pub mail_box_type: MailBoxType,
}

/// Type of mail box
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailBoxType {
    Unbounded,
    Bounded,
}

impl Default for ActorConfig {
    fn default() -> Self {
        Self {
            name: "actor".to_string(),
            buffer_size: 100,
            mail_box_type: MailBoxType::Bounded,
        }
    }
}

/// Builder for creating actor configurations
#[derive(Debug)]
pub struct ActorBuilder {
    config: ActorConfig,
}

impl ActorBuilder {
    pub fn new() -> Self {
        Self {
            config: ActorConfig::default(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn mail_box_type(mut self, mail_box_type: MailBoxType) -> Self {
        self.config.mail_box_type = mail_box_type;
        self
    }

    pub fn build(self) -> ActorConfig {
        self.config
    }
}

impl Default for ActorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Actor handle for sending messages - uses generic channel types
pub struct ActorHandle<T> {
    sender: mpsc::Sender<T>,
}

impl<T> ActorHandle<T> {
    pub fn from_bounded(sender: mpsc::Sender<T>) -> Self {
        Self { sender }
    }

    pub async fn send(&self, msg: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(msg).await
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl<T> fmt::Debug for ActorHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorHandle").finish()
    }
}

/// Actor reference for receiving messages
pub struct ActorRef<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> ActorRef<T> {
    pub fn from_bounded(receiver: mpsc::Receiver<T>) -> Self {
        Self { receiver }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

/// Create a bounded actor channel
pub fn create_bounded_actor_channel<T>(buffer_size: usize) -> (ActorHandle<T>, ActorRef<T>) {
    let (sender, receiver) = mpsc::channel(buffer_size);
    (ActorHandle::from_bounded(sender), ActorRef::from_bounded(receiver))
}

/// Create an unbounded actor channel
pub fn create_unbounded_actor_channel<T>() -> (mpsc::UnboundedSender<T>, mpsc::UnboundedReceiver<T>) {
    mpsc::unbounded_channel()
}

/// Message envelope for actor messages
#[derive(Debug, Clone)]
pub struct ActorMessage<T> {
    pub payload: T,
    pub sender_id: Option<String>,
}

impl<T> ActorMessage<T> {
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            sender_id: None,
        }
    }

    pub fn with_sender_id(mut self, sender_id: &str) -> Self {
        self.sender_id = Some(sender_id.to_string());
        self
    }
}

/// Actor state wrapper
#[derive(Debug)]
pub struct ActorState<S> {
    pub state: S,
    pub message_count: u64,
}

impl<S> ActorState<S> {
    pub fn new(state: S) -> Self {
        Self {
            state,
            message_count: 0,
        }
    }

    pub fn increment(&mut self) {
        self.message_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_config_default() {
        let config = ActorConfig::default();
        assert_eq!(config.name, "actor");
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.mail_box_type, MailBoxType::Bounded);
    }

    #[test]
    fn test_actor_builder() {
        let config = ActorBuilder::new()
            .name("test-actor")
            .buffer_size(200)
            .mail_box_type(MailBoxType::Unbounded)
            .build();
        
        assert_eq!(config.name, "test-actor");
        assert_eq!(config.buffer_size, 200);
        assert_eq!(config.mail_box_type, MailBoxType::Unbounded);
    }

    #[test]
    fn test_actor_message() {
        let msg = ActorMessage::new(42);
        assert_eq!(msg.payload, 42);
        assert!(msg.sender_id.is_none());
    }

    #[test]
    fn test_actor_state() {
        let mut state = ActorState::new(10);
        assert_eq!(state.state, 10);
        assert_eq!(state.message_count, 0);
        
        state.increment();
        assert_eq!(state.message_count, 1);
    }

    #[tokio::test]
    async fn test_actor_channel() {
        let (handle, mut actor_ref) = create_bounded_actor_channel::<i32>(10);
        
        handle.send(1).await.unwrap();
        handle.send(2).await.unwrap();
        
        assert_eq!(actor_ref.recv().await, Some(1));
        assert_eq!(actor_ref.recv().await, Some(2));
    }
}
