//! Notification system for shiplog alerts.
//!
//! Provides a notification system for sending alerts and updates
//! about shiplog events and packet status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Notification priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

/// Notification channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    Email,
    Slack,
    Webhook,
    Console,
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Channel::Email => write!(f, "email"),
            Channel::Slack => write!(f, "slack"),
            Channel::Webhook => write!(f, "webhook"),
            Channel::Console => write!(f, "console"),
        }
    }
}

/// A notification message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub message: String,
    pub priority: Priority,
    pub channel: Channel,
    pub timestamp: DateTime<Utc>,
}

impl Notification {
    /// Create a new notification.
    pub fn new(title: impl Into<String>, message: impl Into<String>, priority: Priority, channel: Channel) -> Self {
        let now = Utc::now();
        let id = format!("notif_{}", now.timestamp_millis());
        Self {
            id,
            title: title.into(),
            message: message.into(),
            priority,
            channel,
            timestamp: now,
        }
    }

    /// Create a low priority notification.
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message, Priority::Low, Channel::Console)
    }

    /// Create a high priority notification.
    pub fn alert(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message, Priority::High, Channel::Console)
    }
}

/// Notifier trait for different notification backends.
pub trait Notifier: Send + Sync {
    /// Send a notification.
    fn notify(&self, notification: &Notification) -> anyhow::Result<()>;

    /// Get the channel this notifier supports.
    fn channel(&self) -> Channel;
}

/// Console notifier for testing and development.
pub struct ConsoleNotifier;

impl Notifier for ConsoleNotifier {
    fn notify(&self, notification: &Notification) -> anyhow::Result<()> {
        println!(
            "[{}] {} ({}) - {}: {}",
            notification.timestamp.format("%Y-%m-%d %H:%M:%S"),
            notification.priority,
            notification.channel,
            notification.title,
            notification.message
        );
        Ok(())
    }

    fn channel(&self) -> Channel {
        Channel::Console
    }
}

/// Notification service for managing notifications.
pub struct NotificationService {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationService {
    /// Create a new notification service.
    pub fn new() -> Self {
        Self {
            notifiers: Vec::new(),
        }
    }

    /// Add a notifier to the service.
    pub fn add_notifier(&mut self, notifier: Box<dyn Notifier>) {
        self.notifiers.push(notifier);
    }

    /// Send a notification to all registered notifiers.
    pub fn send(&self, notification: &Notification) -> anyhow::Result<()> {
        for notifier in &self.notifiers {
            // Only send to matching channel
            if notifier.channel() == notification.channel || notifier.channel() == Channel::Console {
                notifier.notify(notification)?;
            }
        }
        Ok(())
    }

    /// Send a simple notification to console.
    pub fn send_simple(&self, title: &str, message: &str, priority: Priority) -> anyhow::Result<()> {
        let notification = Notification::new(title, message, priority, Channel::Console);
        self.send(&notification)
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        let mut service = Self::new();
        service.add_notifier(Box::new(ConsoleNotifier));
        service
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_priority_display() {
        assert_eq!(format!("{}", Priority::Low), "low");
        assert_eq!(format!("{}", Priority::Critical), "critical");
    }

    #[test]
    fn notification_channel_display() {
        assert_eq!(format!("{}", Channel::Email), "email");
        assert_eq!(format!("{}", Channel::Slack), "slack");
    }

    #[test]
    fn notification_new() {
        let notification = Notification::new(
            "Test Title",
            "Test Message",
            Priority::High,
            Channel::Console,
        );
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.priority, Priority::High);
        assert_eq!(notification.channel, Channel::Console);
    }

    #[test]
    fn notification_info() {
        let notification = Notification::info("Info", "This is an info message");
        assert_eq!(notification.priority, Priority::Low);
    }

    #[test]
    fn notification_alert() {
        let notification = Notification::alert("Alert", "This is an alert");
        assert_eq!(notification.priority, Priority::High);
    }

    #[test]
    fn notification_service_send() {
        let service = NotificationService::default();
        let notification = Notification::info("Test", "Test message");
        // Should not panic
        assert!(service.send(&notification).is_ok());
    }

    #[test]
    fn notification_service_simple() {
        let service = NotificationService::default();
        assert!(service.send_simple("Test", "Message", Priority::Medium).is_ok());
    }

    #[test]
    fn console_notifier_channel() {
        let notifier = ConsoleNotifier;
        assert_eq!(notifier.channel(), Channel::Console);
    }

    #[test]
    fn notification_has_timestamp() {
        let notification = Notification::info("Test", "Test");
        // Timestamp should be close to now (within 1 minute)
        let diff = (Utc::now() - notification.timestamp).num_seconds().abs();
        assert!(diff < 60, "timestamp should be within 60 seconds of now");
    }
}
