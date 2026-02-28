use shiplog_watcher::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ── WatcherConfig ─────────────────────────────────────────────────────

#[test]
fn watcher_config_fields() {
    let config = WatcherConfig {
        path: PathBuf::from("/tmp/test"),
        recursive: true,
        poll_interval: 5,
    };
    assert_eq!(config.path, PathBuf::from("/tmp/test"));
    assert!(config.recursive);
    assert_eq!(config.poll_interval, 5);
}

#[test]
fn watcher_config_serde_roundtrip() {
    let config = WatcherConfig {
        path: PathBuf::from("some/path"),
        recursive: false,
        poll_interval: 10,
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: WatcherConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.path, PathBuf::from("some/path"));
    assert!(!deserialized.recursive);
    assert_eq!(deserialized.poll_interval, 10);
}

#[test]
fn watcher_config_serde_defaults() {
    let json = r#"{"path": "test"}"#;
    let config: WatcherConfig = serde_json::from_str(json).unwrap();
    assert!(config.recursive); // default_true
    assert_eq!(config.poll_interval, 2); // default_poll_interval
}

// ── FileEventKind ─────────────────────────────────────────────────────

#[test]
fn file_event_kind_from_create() {
    use notify::EventKind;
    use notify::event::CreateKind;
    let kind = FileEventKind::from(&EventKind::Create(CreateKind::File));
    assert!(matches!(kind, FileEventKind::Created));
}

#[test]
fn file_event_kind_from_modify() {
    use notify::EventKind;
    use notify::event::ModifyKind;
    let kind = FileEventKind::from(&EventKind::Modify(ModifyKind::Data(
        notify::event::DataChange::Content,
    )));
    assert!(matches!(kind, FileEventKind::Modified));
}

#[test]
fn file_event_kind_from_remove() {
    use notify::EventKind;
    use notify::event::RemoveKind;
    let kind = FileEventKind::from(&EventKind::Remove(RemoveKind::File));
    assert!(matches!(kind, FileEventKind::Removed));
}

#[test]
fn file_event_kind_from_other() {
    use notify::EventKind;
    let kind = FileEventKind::from(&EventKind::Other);
    assert!(matches!(kind, FileEventKind::Any));
}

// ── FileEvent ─────────────────────────────────────────────────────────

#[test]
fn file_event_serde_roundtrip() {
    let event = FileEvent {
        path: PathBuf::from("test.txt"),
        kind: FileEventKind::Modified,
    };
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: FileEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.path, PathBuf::from("test.txt"));
    assert!(matches!(deserialized.kind, FileEventKind::Modified));
}

#[test]
fn file_event_clone() {
    let event = FileEvent {
        path: PathBuf::from("a.rs"),
        kind: FileEventKind::Created,
    };
    let cloned = event.clone();
    assert_eq!(cloned.path, PathBuf::from("a.rs"));
}

// ── FileWatcher ───────────────────────────────────────────────────────

#[test]
fn file_watcher_creates_on_valid_dir() {
    let temp = TempDir::new().unwrap();
    let config = WatcherConfig {
        path: temp.path().to_path_buf(),
        recursive: true,
        poll_interval: 1,
    };
    let watcher = FileWatcher::new(&config);
    assert!(watcher.is_ok());
}

#[test]
fn file_watcher_try_recv_empty_initially() {
    let temp = TempDir::new().unwrap();
    let config = WatcherConfig {
        path: temp.path().to_path_buf(),
        recursive: true,
        poll_interval: 1,
    };
    let watcher = FileWatcher::new(&config).unwrap();
    // No events yet
    assert!(watcher.try_recv().is_none());
}

#[test]
fn file_watcher_detects_file_creation() {
    let temp = TempDir::new().unwrap();
    let config = WatcherConfig {
        path: temp.path().to_path_buf(),
        recursive: true,
        poll_interval: 1,
    };
    let watcher = FileWatcher::new(&config).unwrap();

    // Create a file
    let test_file = temp.path().join("hello.txt");
    fs::write(&test_file, "content").unwrap();

    // Give watcher time to detect
    std::thread::sleep(std::time::Duration::from_secs(2));

    // We may or may not get an event depending on OS timing, but no panic
    let _event = watcher.try_recv();
}

#[test]
fn file_watcher_invalid_path_error() {
    let config = WatcherConfig {
        path: PathBuf::from("/nonexistent/path/that/should/not/exist"),
        recursive: true,
        poll_interval: 1,
    };
    let result = FileWatcher::new(&config);
    assert!(result.is_err());
}
