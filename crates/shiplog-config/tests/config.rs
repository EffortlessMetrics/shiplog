//! Integration tests for shiplog-config.

use shiplog_config::{ConfigFormat, ShiplogConfig, WorkstreamConfig, load_config, save_config};
use std::path::PathBuf;
use tempfile::TempDir;

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    fn arb_workstream() -> impl Strategy<Value = WorkstreamConfig> {
        ("[a-z]{1,20}", "[a-z]{1,10}").prop_map(|(name, source)| WorkstreamConfig {
            name,
            source,
            config: serde_json::json!({}),
            enabled: true,
        })
    }

    fn arb_config() -> impl Strategy<Value = ShiplogConfig> {
        prop::collection::vec(arb_workstream(), 0..5).prop_map(|workstreams| ShiplogConfig {
            output_dir: PathBuf::from("./out"),
            cache_dir: PathBuf::from("./.cache"),
            incremental: true,
            verbose: false,
            workstreams,
        })
    }

    proptest! {
        #[test]
        fn yaml_round_trip(config in arb_config()) {
            let yaml = serde_yaml::to_string(&config).unwrap();
            let loaded: ShiplogConfig = serde_yaml::from_str(&yaml).unwrap();
            prop_assert_eq!(loaded.output_dir, config.output_dir);
            prop_assert_eq!(loaded.cache_dir, config.cache_dir);
            prop_assert_eq!(loaded.incremental, config.incremental);
            prop_assert_eq!(loaded.verbose, config.verbose);
            prop_assert_eq!(loaded.workstreams.len(), config.workstreams.len());
        }

        #[test]
        fn json_round_trip(config in arb_config()) {
            let json = serde_json::to_string(&config).unwrap();
            let loaded: ShiplogConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(loaded.output_dir, config.output_dir);
            prop_assert_eq!(loaded.workstreams.len(), config.workstreams.len());
        }

        #[test]
        fn file_round_trip_yaml(config in arb_config()) {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("config.yaml");
            save_config(&config, &path).unwrap();
            let loaded = load_config(&path).unwrap();
            prop_assert_eq!(loaded.output_dir, config.output_dir);
            prop_assert_eq!(loaded.workstreams.len(), config.workstreams.len());
        }

        #[test]
        fn file_round_trip_json(config in arb_config()) {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("config.json");
            save_config(&config, &path).unwrap();
            let loaded = load_config(&path).unwrap();
            prop_assert_eq!(loaded.output_dir, config.output_dir);
            prop_assert_eq!(loaded.workstreams.len(), config.workstreams.len());
        }
    }
}

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_default_config_yaml() {
    let config = ShiplogConfig::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    insta::assert_snapshot!("default_config_yaml", yaml);
}

#[test]
fn snapshot_default_config_json() {
    let config = ShiplogConfig::default();
    let json = serde_json::to_string_pretty(&config).unwrap();
    insta::assert_snapshot!("default_config_json", json);
}

#[test]
fn snapshot_config_with_workstreams_yaml() {
    let config = ShiplogConfig {
        output_dir: PathBuf::from("/tmp/output"),
        cache_dir: PathBuf::from("/tmp/cache"),
        incremental: false,
        verbose: true,
        workstreams: vec![
            WorkstreamConfig {
                name: "backend".into(),
                source: "github".into(),
                config: serde_json::json!({"repo": "acme/api"}),
                enabled: true,
            },
            WorkstreamConfig {
                name: "frontend".into(),
                source: "gitlab".into(),
                config: serde_json::json!({}),
                enabled: false,
            },
        ],
    };
    let yaml = serde_yaml::to_string(&config).unwrap();
    insta::assert_snapshot!("config_with_workstreams_yaml", yaml);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn default_config_format_is_yaml() {
    assert_eq!(ConfigFormat::default(), ConfigFormat::Yaml);
}

#[test]
fn load_nonexistent_file_errors() {
    let result = load_config("/nonexistent/path/config.yaml");
    assert!(result.is_err());
}

#[test]
fn load_invalid_yaml_errors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.yaml");
    std::fs::write(&path, "{{{{invalid yaml!!!!").unwrap();
    let result = load_config(&path);
    assert!(result.is_err());
}

#[test]
fn load_invalid_json_errors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "not json at all").unwrap();
    let result = load_config(&path);
    assert!(result.is_err());
}

#[test]
fn load_empty_yaml_uses_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("empty.yaml");
    std::fs::write(&path, "{}").unwrap();
    let config = load_config(&path).unwrap();
    // Defaults should kick in for missing fields
    assert_eq!(config.output_dir, PathBuf::from("./packets"));
    assert_eq!(config.cache_dir, PathBuf::from("./.shiplog-cache"));
    assert!(config.incremental);
    assert!(!config.verbose);
    assert!(config.workstreams.is_empty());
}

#[test]
fn load_unknown_extension_defaults_to_yaml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.txt");
    std::fs::write(&path, "output_dir: /custom\nincremental: false\n").unwrap();
    let config = load_config(&path).unwrap();
    assert_eq!(config.output_dir, PathBuf::from("/custom"));
    assert!(!config.incremental);
}

#[test]
fn save_unknown_extension_defaults_to_yaml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.txt");
    let config = ShiplogConfig::default();
    save_config(&config, &path).unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    // YAML output should contain these keys
    assert!(contents.contains("output_dir"));
    assert!(contents.contains("incremental"));
}

#[test]
fn config_with_empty_workstreams_round_trips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.yaml");
    let config = ShiplogConfig {
        workstreams: vec![],
        ..ShiplogConfig::default()
    };
    save_config(&config, &path).unwrap();
    let loaded = load_config(&path).unwrap();
    assert!(loaded.workstreams.is_empty());
}

#[test]
fn workstream_config_preserves_nested_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.json");
    let nested =
        serde_json::json!({"repo": "acme/api", "branch": "main", "nested": {"deep": true}});
    let config = ShiplogConfig {
        workstreams: vec![WorkstreamConfig {
            name: "test".into(),
            source: "github".into(),
            config: nested.clone(),
            enabled: true,
        }],
        ..ShiplogConfig::default()
    };
    save_config(&config, &path).unwrap();
    let loaded = load_config(&path).unwrap();
    assert_eq!(loaded.workstreams[0].config, nested);
}

#[test]
fn yml_extension_detected_as_yaml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.yml");
    let config = ShiplogConfig::default();
    // Save as yaml first (unknown ext -> yaml)
    let yaml = serde_yaml::to_string(&config).unwrap();
    std::fs::write(&path, &yaml).unwrap();
    let loaded = load_config(&path).unwrap();
    assert_eq!(loaded.output_dir, config.output_dir);
}
