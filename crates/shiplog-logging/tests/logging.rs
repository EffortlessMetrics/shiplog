//! Integration tests for shiplog-logging.

use shiplog_logging::*;

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_default_config() {
    let config = LoggingConfig::default();
    insta::assert_yaml_snapshot!("default_logging_config", config);
}

#[test]
fn snapshot_config_with_component_levels() {
    let config = LoggingConfig::new()
        .with_level(LogLevel::Warn)
        .with_format(LogFormat::Json)
        .with_component_level("network", LogLevel::Trace)
        .with_component_level("engine", LogLevel::Debug);
    // Use BTreeMap for deterministic key ordering in snapshots
    let sorted_levels: std::collections::BTreeMap<_, _> = config.component_levels.iter().collect();
    insta::assert_yaml_snapshot!("config_with_components", sorted_levels);
}

#[test]
fn snapshot_log_entry_serialized() {
    let entry = LogEntry {
        timestamp: "2025-01-01T00:00:00+00:00".to_string(),
        level: LogLevel::Error,
        component: Some("ingest".to_string()),
        message: "rate limit exceeded".to_string(),
    };
    insta::assert_yaml_snapshot!("log_entry_serialized", entry);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    fn arb_log_level() -> impl Strategy<Value = LogLevel> {
        prop_oneof![
            Just(LogLevel::Error),
            Just(LogLevel::Warn),
            Just(LogLevel::Info),
            Just(LogLevel::Debug),
            Just(LogLevel::Trace),
        ]
    }

    proptest! {
        #[test]
        fn level_should_log_self(level in arb_log_level()) {
            // Every level should log messages at its own level
            prop_assert!(level.should_log(level));
        }

        #[test]
        fn error_always_logged(level in arb_log_level()) {
            // Error messages should be logged at any level
            prop_assert!(level.should_log(LogLevel::Error));
        }

        #[test]
        fn trace_only_logs_at_trace(level in arb_log_level()) {
            // Only Trace level should log Trace messages
            if level.should_log(LogLevel::Trace) {
                prop_assert_eq!(level, LogLevel::Trace);
            }
        }

        #[test]
        fn log_entry_preserves_message(msg in "[a-zA-Z0-9 ]{1,100}") {
            let entry = LogEntry::new(LogLevel::Info, &msg);
            prop_assert_eq!(&entry.message, &msg);
            prop_assert!(!entry.timestamp.is_empty());
        }

        #[test]
        fn collector_count_matches_pushes(count in 0_usize..20) {
            let mut collector = LogCollector::new();
            for i in 0..count {
                collector.push(LogEntry::new(LogLevel::Info, format!("msg {i}")));
            }
            prop_assert_eq!(collector.entries().len(), count);
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn log_level_ordering_comprehensive() {
    // Error is lowest severity number, Trace is highest
    assert!(LogLevel::Trace.should_log(LogLevel::Error));
    assert!(LogLevel::Trace.should_log(LogLevel::Warn));
    assert!(LogLevel::Trace.should_log(LogLevel::Info));
    assert!(LogLevel::Trace.should_log(LogLevel::Debug));
    assert!(LogLevel::Trace.should_log(LogLevel::Trace));

    assert!(LogLevel::Error.should_log(LogLevel::Error));
    assert!(!LogLevel::Error.should_log(LogLevel::Warn));
    assert!(!LogLevel::Error.should_log(LogLevel::Info));
    assert!(!LogLevel::Error.should_log(LogLevel::Debug));
    assert!(!LogLevel::Error.should_log(LogLevel::Trace));
}

#[test]
fn component_level_overrides_global() {
    let config = LoggingConfig::new()
        .with_level(LogLevel::Error)
        .with_component_level("verbose-module", LogLevel::Trace);

    // Global: only errors
    assert!(!config.should_log(LogLevel::Debug, None));
    assert!(config.should_log(LogLevel::Error, None));

    // Component override: everything
    assert!(config.should_log(LogLevel::Trace, Some("verbose-module")));
    assert!(config.should_log(LogLevel::Debug, Some("verbose-module")));

    // Unknown component falls back to global
    assert!(!config.should_log(LogLevel::Info, Some("unknown-module")));
}

#[test]
fn effective_level_returns_global_for_unknown_component() {
    let config = LoggingConfig::new().with_level(LogLevel::Warn);
    assert_eq!(config.effective_level(None), LogLevel::Warn);
    assert_eq!(config.effective_level(Some("unknown")), LogLevel::Warn);
}

#[test]
fn effective_level_returns_component_override() {
    let config = LoggingConfig::new()
        .with_level(LogLevel::Error)
        .with_component_level("net", LogLevel::Debug);
    assert_eq!(config.effective_level(Some("net")), LogLevel::Debug);
}

#[test]
fn log_collector_empty() {
    let collector = LogCollector::new();
    assert!(collector.entries().is_empty());
    assert!(collector.filter_by_level(LogLevel::Error).is_empty());
}

#[test]
fn log_collector_filter_returns_only_matching() {
    let mut collector = LogCollector::new();
    collector.push(LogEntry::new(LogLevel::Error, "err1"));
    collector.push(LogEntry::new(LogLevel::Info, "info1"));
    collector.push(LogEntry::new(LogLevel::Error, "err2"));
    collector.push(LogEntry::new(LogLevel::Debug, "dbg1"));

    let errors = collector.filter_by_level(LogLevel::Error);
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|e| e.level == LogLevel::Error));

    let debugs = collector.filter_by_level(LogLevel::Debug);
    assert_eq!(debugs.len(), 1);
}

#[test]
fn log_collector_clear() {
    let mut collector = LogCollector::new();
    collector.push(LogEntry::new(LogLevel::Info, "msg"));
    assert_eq!(collector.entries().len(), 1);
    collector.clear();
    assert!(collector.entries().is_empty());
}

#[test]
fn log_entry_with_component_sets_component() {
    let entry = LogEntry::with_component(LogLevel::Warn, "engine", "slow query");
    assert_eq!(entry.component, Some("engine".to_string()));
    assert_eq!(entry.message, "slow query");
    assert_eq!(entry.level, LogLevel::Warn);
}

#[test]
fn log_entry_without_component() {
    let entry = LogEntry::new(LogLevel::Info, "standalone");
    assert!(entry.component.is_none());
}

#[test]
fn logging_config_builder_chain() {
    let config = LoggingConfig::new()
        .with_level(LogLevel::Debug)
        .with_format(LogFormat::Compact)
        .with_component_level("a", LogLevel::Error)
        .with_component_level("b", LogLevel::Trace);

    assert_eq!(config.level, LogLevel::Debug);
    assert_eq!(config.format, LogFormat::Compact);
    assert_eq!(config.component_levels.len(), 2);
}

#[test]
fn default_log_level_is_info() {
    assert_eq!(LogLevel::default(), LogLevel::Info);
}

#[test]
fn default_log_format_is_plain() {
    assert_eq!(LogFormat::default(), LogFormat::Plain);
}

#[test]
fn logging_config_serialization_round_trip() {
    let config = LoggingConfig::new()
        .with_level(LogLevel::Debug)
        .with_format(LogFormat::Json)
        .with_component_level("engine", LogLevel::Trace);

    let json = serde_json::to_string(&config).unwrap();
    let loaded: LoggingConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.level, config.level);
    assert_eq!(loaded.format, config.format);
    assert_eq!(
        loaded.component_levels.get("engine"),
        Some(&LogLevel::Trace)
    );
}
