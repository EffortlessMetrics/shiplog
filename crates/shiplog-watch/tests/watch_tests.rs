use shiplog_watch::*;

#[test]
fn file_event_type_values() {
    assert_eq!(FileEventType::Created, FileEventType::Created);
    assert_ne!(FileEventType::Created, FileEventType::Modified);
    assert_ne!(FileEventType::Modified, FileEventType::Removed);
    assert_ne!(FileEventType::Removed, FileEventType::Renamed);
}

#[test]
fn file_change_creation() {
    let change = FileChange {
        path: std::path::PathBuf::from("test.txt"),
        event_type: FileEventType::Modified,
    };
    assert_eq!(change.path.to_str().unwrap(), "test.txt");
    assert_eq!(change.event_type, FileEventType::Modified);
}

#[test]
fn watch_config_creation() {
    let config = WatchConfig {
        paths: vec![std::path::PathBuf::from(".")],
        recursive: true,
        poll_interval: 100,
    };
    assert!(config.recursive);
    assert_eq!(config.poll_interval, 100);
    assert_eq!(config.paths.len(), 1);
}

#[test]
fn file_event_type_serde_roundtrip() {
    for event_type in [
        FileEventType::Created,
        FileEventType::Modified,
        FileEventType::Removed,
        FileEventType::Renamed,
    ] {
        let json = serde_json::to_string(&event_type).unwrap();
        let de: FileEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, event_type);
    }
}

#[test]
fn watch_config_default_values() {
    let config = WatchConfig::default();
    assert!(config.paths.is_empty());
    assert_eq!(config.poll_interval, 2);
    assert!(config.recursive);
}

#[test]
fn watch_config_multiple_paths() {
    let config = WatchConfig {
        paths: vec![
            std::path::PathBuf::from("/a"),
            std::path::PathBuf::from("/b"),
            std::path::PathBuf::from("/c"),
        ],
        recursive: false,
        poll_interval: 5,
    };
    assert_eq!(config.paths.len(), 3);
    assert!(!config.recursive);
    assert_eq!(config.poll_interval, 5);
}

#[test]
fn file_change_serde_roundtrip() {
    let change = FileChange {
        path: std::path::PathBuf::from("src/main.rs"),
        event_type: FileEventType::Modified,
    };
    let json = serde_json::to_string(&change).unwrap();
    let de: FileChange = serde_json::from_str(&json).unwrap();
    assert_eq!(de.path, change.path);
    assert_eq!(de.event_type, change.event_type);
}

#[test]
fn file_change_all_event_types() {
    for event_type in [
        FileEventType::Created,
        FileEventType::Modified,
        FileEventType::Removed,
        FileEventType::Renamed,
    ] {
        let change = FileChange {
            path: std::path::PathBuf::from("test.txt"),
            event_type,
        };
        assert_eq!(change.event_type, event_type);
    }
}

#[test]
fn file_event_type_clone() {
    let original = FileEventType::Created;
    let cloned = original;
    assert_eq!(original, cloned);
}

#[test]
fn file_change_debug_format() {
    let change = FileChange {
        path: std::path::PathBuf::from("file.txt"),
        event_type: FileEventType::Removed,
    };
    let debug = format!("{change:?}");
    assert!(debug.contains("file.txt"));
    assert!(debug.contains("Removed"));
}

#[test]
fn watch_config_non_recursive() {
    let config = WatchConfig {
        paths: vec![std::path::PathBuf::from(".")],
        recursive: false,
        poll_interval: 10,
    };
    assert!(!config.recursive);
    assert_eq!(config.poll_interval, 10);
}

#[test]
fn watch_directory_creates_watcher() {
    let dir = tempfile::TempDir::new().unwrap();
    let watcher = watch_directory(dir.path());
    assert!(watcher.is_ok());
}

#[test]
fn file_watcher_try_recv_returns_none_when_no_events() {
    let dir = tempfile::TempDir::new().unwrap();
    let watcher = FileWatcher::new(&WatchConfig {
        paths: vec![dir.path().to_path_buf()],
        poll_interval: 1,
        recursive: true,
    })
    .unwrap();
    assert!(watcher.try_recv().is_none());
}

#[test]
fn file_watcher_recv_timeout_returns_none_on_timeout() {
    let dir = tempfile::TempDir::new().unwrap();
    let watcher = FileWatcher::new(&WatchConfig {
        paths: vec![dir.path().to_path_buf()],
        poll_interval: 1,
        recursive: false,
    })
    .unwrap();
    let result = watcher.recv_timeout(std::time::Duration::from_millis(50));
    assert!(result.is_none());
}
