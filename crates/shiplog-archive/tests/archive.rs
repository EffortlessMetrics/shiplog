use chrono::{Duration, Utc};
use shiplog_archive::{
    ArchiveConfig, ArchiveEntry, ArchiveFormat, ArchiveManifest, ArchiveState, ArchiveStatus,
};
use std::path::PathBuf;

// ── ArchiveFormat ─────────────────────────────────────────────────

#[test]
fn archive_format_eq() {
    assert_eq!(ArchiveFormat::Zip, ArchiveFormat::Zip);
    assert_eq!(ArchiveFormat::Gzip, ArchiveFormat::Gzip);
    assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Gzip);
}

#[test]
fn archive_format_serde_roundtrip() {
    for fmt in [ArchiveFormat::Zip, ArchiveFormat::Gzip] {
        let json = serde_json::to_string(&fmt).unwrap();
        let de: ArchiveFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(de, fmt);
    }
}

// ── ArchiveConfig ─────────────────────────────────────────────────

#[test]
fn config_default() {
    let cfg = ArchiveConfig::default();
    assert_eq!(cfg.format, ArchiveFormat::Zip);
    assert_eq!(cfg.retention_days, 90);
    assert_eq!(cfg.compression_level, Some(6));
}

#[test]
fn config_serde_roundtrip() {
    let cfg = ArchiveConfig {
        format: ArchiveFormat::Gzip,
        retention_days: 30,
        compression_level: None,
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let de: ArchiveConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(de.format, ArchiveFormat::Gzip);
    assert_eq!(de.retention_days, 30);
    assert_eq!(de.compression_level, None);
}

#[test]
fn config_clone() {
    let cfg = ArchiveConfig::default();
    let c2 = cfg.clone();
    assert_eq!(c2.retention_days, cfg.retention_days);
}

// ── ArchiveEntry ──────────────────────────────────────────────────

#[test]
fn entry_creation() {
    let entry = ArchiveEntry {
        original_path: PathBuf::from("packet.md"),
        archived_at: Utc::now(),
        size_original: 5000,
        size_compressed: 2000,
        checksum: "abc".into(),
    };
    assert_eq!(entry.size_original, 5000);
    assert_eq!(entry.size_compressed, 2000);
}

#[test]
fn entry_serde_roundtrip() {
    let entry = ArchiveEntry {
        original_path: PathBuf::from("data/file.json"),
        archived_at: Utc::now(),
        size_original: 100,
        size_compressed: 50,
        checksum: "deadbeef".into(),
    };
    let json = serde_json::to_string(&entry).unwrap();
    let de: ArchiveEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(de.original_path, PathBuf::from("data/file.json"));
    assert_eq!(de.checksum, "deadbeef");
}

// ── ArchiveManifest ───────────────────────────────────────────────

#[test]
fn manifest_new_is_empty() {
    let m = ArchiveManifest::new();
    assert!(m.entries.is_empty());
    assert_eq!(m.total_original_size, 0);
    assert_eq!(m.total_compressed_size, 0);
}

#[test]
fn manifest_default_is_empty() {
    let m = ArchiveManifest::default();
    assert!(m.entries.is_empty());
}

#[test]
fn manifest_add_entry_accumulates() {
    let mut m = ArchiveManifest::new();
    m.add_entry(make_entry(1000, 400));
    m.add_entry(make_entry(2000, 800));
    assert_eq!(m.entries.len(), 2);
    assert_eq!(m.total_original_size, 3000);
    assert_eq!(m.total_compressed_size, 1200);
}

#[test]
fn manifest_compression_ratio_normal() {
    let mut m = ArchiveManifest::new();
    m.add_entry(make_entry(1000, 500));
    assert!((m.compression_ratio() - 50.0).abs() < f64::EPSILON);
}

#[test]
fn manifest_compression_ratio_empty() {
    let m = ArchiveManifest::new();
    assert!((m.compression_ratio() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn manifest_compression_ratio_no_compression() {
    let mut m = ArchiveManifest::new();
    m.add_entry(make_entry(1000, 1000));
    assert!((m.compression_ratio() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn manifest_compression_ratio_full_compression() {
    let mut m = ArchiveManifest::new();
    m.add_entry(make_entry(1000, 0));
    assert!((m.compression_ratio() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn manifest_serde_roundtrip() {
    let mut m = ArchiveManifest::new();
    m.add_entry(make_entry(100, 50));
    let json = serde_json::to_string(&m).unwrap();
    let de: ArchiveManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.entries.len(), 1);
    assert_eq!(de.total_original_size, 100);
}

// ── ArchiveStatus ─────────────────────────────────────────────────

#[test]
fn status_eq() {
    assert_eq!(ArchiveStatus::Pending, ArchiveStatus::Pending);
    assert_eq!(ArchiveStatus::InProgress, ArchiveStatus::InProgress);
    assert_eq!(ArchiveStatus::Completed, ArchiveStatus::Completed);
    assert_eq!(
        ArchiveStatus::Failed("err".into()),
        ArchiveStatus::Failed("err".into())
    );
    assert_ne!(ArchiveStatus::Pending, ArchiveStatus::Completed);
}

#[test]
fn status_serde_roundtrip() {
    let statuses = [
        ArchiveStatus::Pending,
        ArchiveStatus::InProgress,
        ArchiveStatus::Completed,
        ArchiveStatus::Failed("disk full".into()),
    ];
    for s in &statuses {
        let json = serde_json::to_string(s).unwrap();
        let de: ArchiveStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(&de, s);
    }
}

// ── ArchiveState ──────────────────────────────────────────────────

#[test]
fn state_new_is_pending() {
    let state = ArchiveState::new(ArchiveConfig::default());
    assert_eq!(state.status, ArchiveStatus::Pending);
}

#[test]
fn state_start_transitions_to_in_progress() {
    let mut state = ArchiveState::new(ArchiveConfig::default());
    state.start();
    assert_eq!(state.status, ArchiveStatus::InProgress);
}

#[test]
fn state_complete_transitions() {
    let mut state = ArchiveState::new(ArchiveConfig::default());
    state.start();
    let mut manifest = ArchiveManifest::new();
    manifest.add_entry(make_entry(500, 200));
    state.complete(manifest);
    assert_eq!(state.status, ArchiveStatus::Completed);
    assert_eq!(state.manifest.entries.len(), 1);
}

#[test]
fn state_fail_transitions() {
    let mut state = ArchiveState::new(ArchiveConfig::default());
    state.fail("IO error".into());
    match &state.status {
        ArchiveStatus::Failed(msg) => assert_eq!(msg, "IO error"),
        other => panic!("Expected Failed, got {:?}", other),
    }
}

#[test]
fn should_archive_old_packet() {
    let config = ArchiveConfig {
        retention_days: 30,
        ..ArchiveConfig::default()
    };
    let state = ArchiveState::new(config);
    let old = Utc::now() - Duration::days(60);
    assert!(state.should_archive(&old));
}

#[test]
fn should_not_archive_recent_packet() {
    let config = ArchiveConfig {
        retention_days: 30,
        ..ArchiveConfig::default()
    };
    let state = ArchiveState::new(config);
    let recent = Utc::now() - Duration::days(10);
    assert!(!state.should_archive(&recent));
}

#[test]
fn should_archive_exactly_at_boundary() {
    let config = ArchiveConfig {
        retention_days: 30,
        ..ArchiveConfig::default()
    };
    let state = ArchiveState::new(config);
    let boundary = Utc::now() - Duration::days(30);
    assert!(state.should_archive(&boundary));
}

#[test]
fn should_archive_zero_retention() {
    let config = ArchiveConfig {
        retention_days: 0,
        ..ArchiveConfig::default()
    };
    let state = ArchiveState::new(config);
    let now = Utc::now();
    assert!(state.should_archive(&now));
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn manifest_totals_are_sum(
            sizes in prop::collection::vec((1u64..10_000, 1u64..10_000), 1..20)
        ) {
            let mut m = ArchiveManifest::new();
            let mut exp_orig = 0u64;
            let mut exp_comp = 0u64;
            for (orig, comp) in &sizes {
                m.add_entry(make_entry(*orig, *comp));
                exp_orig += orig;
                exp_comp += comp;
            }
            prop_assert_eq!(m.total_original_size, exp_orig);
            prop_assert_eq!(m.total_compressed_size, exp_comp);
            prop_assert_eq!(m.entries.len(), sizes.len());
        }

        #[test]
        fn compression_ratio_range(orig in 1u64..100_000, comp in 0u64..100_000) {
            let mut m = ArchiveManifest::new();
            m.add_entry(make_entry(orig, comp));
            let ratio = m.compression_ratio();
            prop_assert!(ratio >= 0.0);
        }

        #[test]
        fn retention_old_always_archived(days in 1u32..365, extra in 0i64..365) {
            let config = ArchiveConfig {
                retention_days: days,
                ..ArchiveConfig::default()
            };
            let state = ArchiveState::new(config);
            let old = Utc::now() - Duration::days(days as i64 + extra);
            prop_assert!(state.should_archive(&old));
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn make_entry(orig: u64, comp: u64) -> ArchiveEntry {
    ArchiveEntry {
        original_path: PathBuf::from("file.dat"),
        archived_at: Utc::now(),
        size_original: orig,
        size_compressed: comp,
        checksum: "test".into(),
    }
}
