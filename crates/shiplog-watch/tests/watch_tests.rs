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
