use proptest::prelude::*;
use shiplog_cron::*;
use std::path::PathBuf;

// ── CronConfig ──────────────────────────────────────────────────────

#[test]
fn cron_config_basic() {
    let c = CronConfig {
        schedule: "0 0 * * 0".to_string(),
        incremental: true,
        output_dir: PathBuf::from("/tmp/shiplog"),
    };
    assert_eq!(c.schedule, "0 0 * * 0");
    assert!(c.incremental);
    assert_eq!(c.output_dir, PathBuf::from("/tmp/shiplog"));
}

#[test]
fn cron_config_non_incremental() {
    let c = CronConfig {
        schedule: "*/5 * * * *".to_string(),
        incremental: false,
        output_dir: PathBuf::from("."),
    };
    assert!(!c.incremental);
}

// ── CronScheduler ───────────────────────────────────────────────────

#[test]
fn scheduler_construction() {
    let c = CronConfig {
        schedule: "0 * * * *".to_string(),
        incremental: true,
        output_dir: PathBuf::from("/out"),
    };
    let _scheduler = CronScheduler::new(c);
}

// ── Serde round-trip ────────────────────────────────────────────────

#[test]
fn config_serde_roundtrip() {
    let c = CronConfig {
        schedule: "0 0 * * 1".to_string(),
        incremental: true,
        output_dir: PathBuf::from("/data"),
    };
    let json = serde_json::to_string(&c).unwrap();
    let c2: CronConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(c2.schedule, c.schedule);
    assert_eq!(c2.incremental, c.incremental);
    assert_eq!(c2.output_dir, c.output_dir);
}

#[test]
fn config_deserializes_with_defaults() {
    let json = r#"{"schedule":"@daily","output_dir":"/out"}"#;
    let c: CronConfig = serde_json::from_str(json).unwrap();
    assert!(c.incremental); // default is true
}

#[test]
fn config_deserializes_incremental_false() {
    let json = r#"{"schedule":"@hourly","incremental":false,"output_dir":"."}"#;
    let c: CronConfig = serde_json::from_str(json).unwrap();
    assert!(!c.incremental);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn empty_schedule_string() {
    let c = CronConfig {
        schedule: String::new(),
        incremental: true,
        output_dir: PathBuf::new(),
    };
    assert!(c.schedule.is_empty());
}

#[test]
fn output_dir_empty_path() {
    let c = CronConfig {
        schedule: "* * * * *".to_string(),
        incremental: false,
        output_dir: PathBuf::new(),
    };
    assert_eq!(c.output_dir, PathBuf::new());
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn serde_roundtrip_preserves_schedule(schedule in "[a-zA-Z0-9 */,\\-]{1,50}") {
        let c = CronConfig {
            schedule: schedule.clone(),
            incremental: true,
            output_dir: PathBuf::from("/tmp"),
        };
        let json = serde_json::to_string(&c).unwrap();
        let c2: CronConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(c2.schedule, schedule);
    }

    #[test]
    fn serde_roundtrip_preserves_incremental(inc in proptest::bool::ANY) {
        let c = CronConfig {
            schedule: "0 0 * * *".to_string(),
            incremental: inc,
            output_dir: PathBuf::from("/tmp"),
        };
        let json = serde_json::to_string(&c).unwrap();
        let c2: CronConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(c2.incremental, inc);
    }

    #[test]
    fn scheduler_can_be_created_with_any_schedule(schedule in ".{1,100}") {
        let c = CronConfig {
            schedule,
            incremental: true,
            output_dir: PathBuf::from("/tmp"),
        };
        let _s = CronScheduler::new(c);
    }
}
