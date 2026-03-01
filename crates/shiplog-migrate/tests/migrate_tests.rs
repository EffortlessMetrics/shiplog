use shiplog_migrate::*;

#[test]
fn migration_creation() {
    let m = Migration::new(1, 2, "add_tags");
    assert_eq!(m.from_version, 1);
    assert_eq!(m.to_version, 2);
    assert_eq!(m.name, "add_tags");
    assert!(m.description.is_empty());
}

#[test]
fn migration_builder_methods() {
    let m = Migration::new(1, 2, "add_tags")
        .with_description("Add tags column")
        .with_up_script("ALTER TABLE events ADD tags TEXT")
        .with_down_script("ALTER TABLE events DROP tags");
    assert_eq!(m.description, "Add tags column");
    assert!(m.up_script.contains("ADD tags"));
    assert!(m.down_script.contains("DROP tags"));
}

#[test]
fn migration_status_values() {
    assert_eq!(MigrationStatus::Pending, MigrationStatus::Pending);
    assert_ne!(MigrationStatus::Pending, MigrationStatus::Completed);
    assert_ne!(MigrationStatus::InProgress, MigrationStatus::Failed);
}

#[test]
fn migration_status_serde_roundtrip() {
    for status in [
        MigrationStatus::Pending,
        MigrationStatus::InProgress,
        MigrationStatus::Completed,
        MigrationStatus::Failed,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let de: MigrationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, status);
    }
}

#[test]
fn migration_serde_roundtrip() {
    let m = Migration::new(1, 2, "test")
        .with_description("desc")
        .with_up_script("up")
        .with_down_script("down");
    let json = serde_json::to_string(&m).unwrap();
    let de: Migration = serde_json::from_str(&json).unwrap();
    assert_eq!(de.from_version, 1);
    assert_eq!(de.to_version, 2);
    assert_eq!(de.name, "test");
}

#[test]
fn migration_record_creation() {
    let record = MigrationRecord {
        version: 1,
        name: "init".to_string(),
        applied_at: chrono::Utc::now(),
        status: MigrationStatus::Completed,
    };
    assert_eq!(record.version, 1);
    assert_eq!(record.status, MigrationStatus::Completed);
}
