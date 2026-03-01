use shiplog_manual_events::*;
use shiplog_schema::event::{ManualDate, ManualEventType};

#[test]
fn create_empty_file_has_version_1() {
    let file = create_empty_file();
    assert_eq!(file.version, 1);
    assert!(file.events.is_empty());
}

#[test]
fn create_entry_basic() {
    let entry = create_entry(
        "test-1",
        ManualEventType::Note,
        ManualDate::Single(chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
        "My note",
    );
    assert_eq!(entry.id, "test-1");
    assert_eq!(entry.title, "My note");
    assert!(entry.description.is_none());
    assert!(entry.workstream.is_none());
    assert!(entry.tags.is_empty());
}

#[test]
fn read_write_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manual.yaml");

    let mut file = create_empty_file();
    file.events.push(create_entry(
        "e1",
        ManualEventType::Note,
        ManualDate::Single(chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
        "Title",
    ));

    write_manual_events(&path, &file).unwrap();
    let loaded = read_manual_events(&path).unwrap();
    assert_eq!(loaded.events.len(), 1);
    assert_eq!(loaded.events[0].id, "e1");
    assert_eq!(loaded.events[0].title, "Title");
}

#[test]
fn read_nonexistent_file_errors() {
    let result = read_manual_events(std::path::Path::new("/nonexistent/path.yaml"));
    assert!(result.is_err());
}
