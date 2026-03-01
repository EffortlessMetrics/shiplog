use shiplog_backup::{BackupArchive, BackupManager, BackupMetadata};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── BackupMetadata ────────────────────────────────────────────────

#[test]
fn metadata_serde_roundtrip() {
    let meta = BackupMetadata {
        id: "test-001".into(),
        created_at: chrono::Utc::now(),
        description: Some("desc".into()),
        size_bytes: 1024,
        file_count: 3,
        checksum: "sha256hex".into(),
    };
    let json = serde_json::to_string(&meta).unwrap();
    let de: BackupMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, "test-001");
    assert_eq!(de.description, Some("desc".into()));
    assert_eq!(de.size_bytes, 1024);
    assert_eq!(de.file_count, 3);
}

#[test]
fn metadata_no_description() {
    let meta = BackupMetadata {
        id: "no-desc".into(),
        created_at: chrono::Utc::now(),
        description: None,
        size_bytes: 0,
        file_count: 0,
        checksum: String::new(),
    };
    assert!(meta.description.is_none());
}

// ── BackupArchive ─────────────────────────────────────────────────

#[test]
fn archive_new() {
    let a = BackupArchive::new("id1", Some("desc".into()), "/tmp/a.zip");
    assert_eq!(a.metadata.id, "id1");
    assert_eq!(a.path(), Path::new("/tmp/a.zip"));
    assert_eq!(a.metadata.size_bytes, 0);
    assert_eq!(a.metadata.file_count, 0);
}

#[test]
fn archive_set_metadata() {
    let mut a = BackupArchive::new("id2", None, "/tmp/b.zip");
    a.set_metadata(500, 10, "abc123");
    assert_eq!(a.metadata.size_bytes, 500);
    assert_eq!(a.metadata.file_count, 10);
    assert_eq!(a.metadata.checksum, "abc123");
}

// ── BackupManager: list on empty dir ──────────────────────────────

#[test]
fn manager_list_empty() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path());
    let backups = mgr.list_backups().unwrap();
    assert!(backups.is_empty());
}

#[test]
fn manager_list_nonexistent_dir() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path().join("does_not_exist"));
    let backups = mgr.list_backups().unwrap();
    assert!(backups.is_empty());
}

// ── BackupManager: create backup ──────────────────────────────────

#[test]
fn create_backup_single_file() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    fs::write(source_dir.join("hello.txt"), "Hello!").unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "test").unwrap();

    assert!(backup.path().exists());
    assert_eq!(backup.metadata.file_count, 1);
    assert!(!backup.metadata.checksum.is_empty());
}

#[test]
fn create_backup_multiple_files() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    fs::write(source_dir.join("a.txt"), "aaa").unwrap();
    fs::write(source_dir.join("b.txt"), "bbb").unwrap();
    fs::write(source_dir.join("c.json"), r#"{"c":1}"#).unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "multi").unwrap();

    assert_eq!(backup.metadata.file_count, 3);
}

#[test]
fn create_backup_nested_dirs() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    fs::create_dir_all(source_dir.join("sub/deep")).unwrap();
    fs::write(source_dir.join("root.txt"), "root").unwrap();
    fs::write(source_dir.join("sub/mid.txt"), "mid").unwrap();
    fs::write(source_dir.join("sub/deep/leaf.txt"), "leaf").unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "nested").unwrap();

    assert_eq!(backup.metadata.file_count, 3);
}

#[test]
fn create_backup_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "empty").unwrap();

    assert_eq!(backup.metadata.file_count, 0);
    assert!(backup.path().exists());
}

#[test]
fn create_backup_binary_file() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    let binary_data: Vec<u8> = (0..=255).collect();
    fs::write(source_dir.join("data.bin"), &binary_data).unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "binary").unwrap();

    assert_eq!(backup.metadata.file_count, 1);
}

#[test]
fn create_backup_large_file() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    let big_data = vec![0xABu8; 50_000];
    fs::write(source_dir.join("big.dat"), &big_data).unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "large").unwrap();

    assert_eq!(backup.metadata.file_count, 1);
    assert!(backup.metadata.size_bytes >= 50_000);
}

// ── BackupManager: restore ────────────────────────────────────────

#[test]
fn restore_backup_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());
    let target_dir = tmp.path().join("restored");

    fs::write(source_dir.join("file.txt"), "content").unwrap();
    fs::write(source_dir.join("data.json"), r#"{"k":"v"}"#).unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "restore-test").unwrap();

    mgr.restore_backup(&backup.metadata.id, &target_dir)
        .unwrap();

    assert_eq!(
        fs::read_to_string(target_dir.join("file.txt")).unwrap(),
        "content"
    );
    assert_eq!(
        fs::read_to_string(target_dir.join("data.json")).unwrap(),
        r#"{"k":"v"}"#
    );
}

#[test]
fn restore_backup_nested_structure() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());
    let target_dir = tmp.path().join("restored");

    fs::create_dir_all(source_dir.join("a/b")).unwrap();
    fs::write(source_dir.join("a/b/deep.txt"), "deep").unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "nested-restore").unwrap();

    mgr.restore_backup(&backup.metadata.id, &target_dir)
        .unwrap();

    assert_eq!(
        fs::read_to_string(target_dir.join("a/b/deep.txt")).unwrap(),
        "deep"
    );
}

#[test]
fn restore_backup_binary_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());
    let target_dir = tmp.path().join("restored");

    let data: Vec<u8> = (0..=255).collect();
    fs::write(source_dir.join("bin.dat"), &data).unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "bin-restore").unwrap();

    mgr.restore_backup(&backup.metadata.id, &target_dir)
        .unwrap();

    assert_eq!(fs::read(target_dir.join("bin.dat")).unwrap(), data);
}

#[test]
fn restore_nonexistent_backup_fails() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path());
    let result = mgr.restore_backup("nonexistent", &tmp.path().join("target"));
    assert!(result.is_err());
}

// ── BackupManager: delete ─────────────────────────────────────────

#[test]
fn delete_backup_removes_files() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    fs::write(source_dir.join("f.txt"), "x").unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "del-test").unwrap();
    let id = backup.metadata.id.clone();

    assert!(backup_dir.join(format!("{id}.zip")).exists());
    assert!(backup_dir.join(format!("{id}.meta.json")).exists());

    mgr.delete_backup(&id).unwrap();

    assert!(!backup_dir.join(format!("{id}.zip")).exists());
    assert!(!backup_dir.join(format!("{id}.meta.json")).exists());
}

#[test]
fn delete_nonexistent_backup_is_ok() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path());
    mgr.delete_backup("nonexistent").unwrap();
}

// ── BackupManager: get_backup ─────────────────────────────────────

#[test]
fn get_backup_metadata() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    fs::write(source_dir.join("f.txt"), "data").unwrap();

    let mgr = BackupManager::new(&backup_dir);
    let backup = mgr.create_backup(&source_dir, "get-test").unwrap();
    let id = backup.metadata.id.clone();

    let meta = mgr.get_backup(&id).unwrap();
    assert_eq!(meta.id, id);
    assert_eq!(meta.file_count, 1);
}

#[test]
fn get_nonexistent_backup_fails() {
    let tmp = TempDir::new().unwrap();
    let mgr = BackupManager::new(tmp.path());
    assert!(mgr.get_backup("ghost").is_err());
}

// ── BackupManager: list backups ───────────────────────────────────

#[test]
fn list_backups_returns_all() {
    let tmp = TempDir::new().unwrap();
    let (backup_dir, source_dir) = setup_dirs(tmp.path());

    fs::write(source_dir.join("f.txt"), "x").unwrap();

    let mgr = BackupManager::new(&backup_dir);
    mgr.create_backup(&source_dir, "a").unwrap();
    mgr.create_backup(&source_dir, "b").unwrap();

    let list = mgr.list_backups().unwrap();
    assert_eq!(list.len(), 2);
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn backup_restore_roundtrip(content in prop::collection::vec(any::<u8>(), 0..1000)) {
            let tmp = TempDir::new().unwrap();
            let (backup_dir, source_dir) = setup_dirs(tmp.path());
            let target_dir = tmp.path().join("restored");

            fs::write(source_dir.join("data.bin"), &content).unwrap();

            let mgr = BackupManager::new(&backup_dir);
            let backup = mgr.create_backup(&source_dir, "prop").unwrap();
            mgr.restore_backup(&backup.metadata.id, &target_dir).unwrap();

            let restored = fs::read(target_dir.join("data.bin")).unwrap();
            prop_assert_eq!(restored, content);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn setup_dirs(base: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let backup_dir = base.join("backups");
    let source_dir = base.join("source");
    fs::create_dir_all(&source_dir).unwrap();
    (backup_dir, source_dir)
}
