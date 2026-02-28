use shiplog_notify::{
    Channel, ConsoleNotifier, Notification, NotificationService, Notifier, Priority,
};
use std::sync::{Arc, Mutex};

// --- Priority tests ---

#[test]
fn priority_display() {
    assert_eq!(format!("{}", Priority::Low), "low");
    assert_eq!(format!("{}", Priority::Medium), "medium");
    assert_eq!(format!("{}", Priority::High), "high");
    assert_eq!(format!("{}", Priority::Critical), "critical");
}

#[test]
fn priority_equality() {
    assert_eq!(Priority::Low, Priority::Low);
    assert_ne!(Priority::Low, Priority::High);
}

// --- Channel tests ---

#[test]
fn channel_display() {
    assert_eq!(format!("{}", Channel::Email), "email");
    assert_eq!(format!("{}", Channel::Slack), "slack");
    assert_eq!(format!("{}", Channel::Webhook), "webhook");
    assert_eq!(format!("{}", Channel::Console), "console");
}

#[test]
fn channel_equality() {
    assert_eq!(Channel::Console, Channel::Console);
    assert_ne!(Channel::Email, Channel::Slack);
}

// --- Notification tests ---

#[test]
fn notification_new() {
    let n = Notification::new("Title", "Body", Priority::High, Channel::Email);
    assert_eq!(n.title, "Title");
    assert_eq!(n.message, "Body");
    assert_eq!(n.priority, Priority::High);
    assert_eq!(n.channel, Channel::Email);
    assert!(n.id.starts_with("notif_"));
}

#[test]
fn notification_info() {
    let n = Notification::info("Info Title", "Info message");
    assert_eq!(n.priority, Priority::Low);
    assert_eq!(n.channel, Channel::Console);
    assert_eq!(n.title, "Info Title");
}

#[test]
fn notification_alert() {
    let n = Notification::alert("Alert!", "Something bad");
    assert_eq!(n.priority, Priority::High);
    assert_eq!(n.channel, Channel::Console);
}

#[test]
fn notification_has_recent_timestamp() {
    let n = Notification::info("T", "M");
    let diff = (chrono::Utc::now() - n.timestamp).num_seconds().abs();
    assert!(diff < 5);
}

#[test]
fn notification_clone() {
    let n = Notification::new("T", "M", Priority::Critical, Channel::Webhook);
    let cloned = n.clone();
    assert_eq!(cloned.title, n.title);
    assert_eq!(cloned.id, n.id);
}

// --- ConsoleNotifier tests ---

#[test]
fn console_notifier_channel() {
    let notifier = ConsoleNotifier;
    assert_eq!(notifier.channel(), Channel::Console);
}

#[test]
fn console_notifier_notify_ok() {
    let notifier = ConsoleNotifier;
    let n = Notification::info("Test", "Message");
    assert!(notifier.notify(&n).is_ok());
}

// --- Custom notifier for testing ---

struct RecordingNotifier {
    channel: Channel,
    log: Arc<Mutex<Vec<String>>>,
}

impl RecordingNotifier {
    fn new(channel: Channel, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self { channel, log }
    }
}

impl Notifier for RecordingNotifier {
    fn notify(&self, notification: &Notification) -> anyhow::Result<()> {
        self.log.lock().unwrap().push(format!(
            "[{}] {}",
            notification.priority, notification.title
        ));
        Ok(())
    }
    fn channel(&self) -> Channel {
        self.channel
    }
}

struct FailingNotifier;

impl Notifier for FailingNotifier {
    fn notify(&self, _notification: &Notification) -> anyhow::Result<()> {
        anyhow::bail!("notification failed")
    }
    fn channel(&self) -> Channel {
        Channel::Console
    }
}

// --- NotificationService tests ---

#[test]
fn service_new_is_empty() {
    let service = NotificationService::new();
    // No notifiers, so send should still succeed (no-op)
    let n = Notification::info("T", "M");
    assert!(service.send(&n).is_ok());
}

#[test]
fn service_default_has_console_notifier() {
    let service = NotificationService::default();
    let n = Notification::info("T", "M");
    assert!(service.send(&n).is_ok());
}

#[test]
fn service_add_notifier_and_send() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut service = NotificationService::new();
    service.add_notifier(Box::new(RecordingNotifier::new(
        Channel::Console,
        log.clone(),
    )));

    let n = Notification::new("Hello", "World", Priority::Medium, Channel::Console);
    assert!(service.send(&n).is_ok());

    let entries = log.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], "[medium] Hello");
}

#[test]
fn service_send_matches_channel() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut service = NotificationService::new();

    // Email notifier
    service.add_notifier(Box::new(RecordingNotifier::new(
        Channel::Email,
        log.clone(),
    )));

    // Send email notification -> should match
    let n = Notification::new("T", "M", Priority::Low, Channel::Email);
    service.send(&n).unwrap();
    assert_eq!(log.lock().unwrap().len(), 1);

    // Console notifier always receives
    let console_log = Arc::new(Mutex::new(Vec::new()));
    service.add_notifier(Box::new(RecordingNotifier::new(
        Channel::Console,
        console_log.clone(),
    )));

    let n2 = Notification::new("T2", "M2", Priority::High, Channel::Slack);
    service.send(&n2).unwrap();
    // Console notifier matches any channel
    assert_eq!(console_log.lock().unwrap().len(), 1);
}

#[test]
fn service_send_simple() {
    let service = NotificationService::default();
    assert!(service.send_simple("T", "M", Priority::Low).is_ok());
}

#[test]
fn service_failing_notifier_propagates_error() {
    let mut service = NotificationService::new();
    service.add_notifier(Box::new(FailingNotifier));

    let n = Notification::info("T", "M");
    assert!(service.send(&n).is_err());
}

#[test]
fn service_multiple_notifiers() {
    let log1 = Arc::new(Mutex::new(Vec::new()));
    let log2 = Arc::new(Mutex::new(Vec::new()));
    let mut service = NotificationService::new();

    service.add_notifier(Box::new(RecordingNotifier::new(
        Channel::Console,
        log1.clone(),
    )));
    service.add_notifier(Box::new(RecordingNotifier::new(
        Channel::Console,
        log2.clone(),
    )));

    let n = Notification::info("T", "M");
    service.send(&n).unwrap();

    assert_eq!(log1.lock().unwrap().len(), 1);
    assert_eq!(log2.lock().unwrap().len(), 1);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn notification_title_preserved(title in "\\PC{1,50}") {
            let n = Notification::new(&title, "msg", Priority::Low, Channel::Console);
            prop_assert_eq!(&n.title, &title);
        }

        #[test]
        fn notification_message_preserved(msg in "\\PC{1,100}") {
            let n = Notification::info("T", &msg);
            prop_assert_eq!(&n.message, &msg);
        }

        #[test]
        fn priority_roundtrip(idx in 0u8..4) {
            let p = match idx {
                0 => Priority::Low,
                1 => Priority::Medium,
                2 => Priority::High,
                _ => Priority::Critical,
            };
            let cloned = p;
            prop_assert_eq!(p, cloned);
        }
    }
}
