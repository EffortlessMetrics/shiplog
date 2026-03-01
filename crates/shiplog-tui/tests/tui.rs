use shiplog_tui::*;
use std::path::PathBuf;

// ── TuiConfig ─────────────────────────────────────────────────────────

#[test]
fn tui_config_fields() {
    let config = TuiConfig {
        workstreams_path: PathBuf::from("workstreams.yaml"),
        show_receipts: true,
    };
    assert_eq!(config.workstreams_path, PathBuf::from("workstreams.yaml"));
    assert!(config.show_receipts);
}

#[test]
fn tui_config_clone() {
    let config = TuiConfig {
        workstreams_path: PathBuf::from("path"),
        show_receipts: false,
    };
    let cloned = config.clone();
    assert_eq!(cloned.workstreams_path, PathBuf::from("path"));
    assert!(!cloned.show_receipts);
}

#[test]
fn tui_config_serde_roundtrip() {
    let config = TuiConfig {
        workstreams_path: PathBuf::from("ws.yaml"),
        show_receipts: false,
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: TuiConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.workstreams_path, PathBuf::from("ws.yaml"));
    assert!(!deserialized.show_receipts);
}

#[test]
fn tui_config_serde_defaults() {
    let json = r#"{}"#;
    let config: TuiConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.workstreams_path, PathBuf::default());
    assert!(config.show_receipts); // default_true
}

#[test]
fn tui_config_debug() {
    let config = TuiConfig {
        workstreams_path: PathBuf::from("test"),
        show_receipts: true,
    };
    let debug = format!("{:?}", config);
    assert!(debug.contains("test"));
    assert!(debug.contains("true"));
}

// ── TuiMode ───────────────────────────────────────────────────────────

#[test]
fn tui_mode_equality() {
    assert_eq!(TuiMode::List, TuiMode::List);
    assert_eq!(TuiMode::EditTitle, TuiMode::EditTitle);
    assert_eq!(TuiMode::EditSummary, TuiMode::EditSummary);
    assert_eq!(TuiMode::EditReceipts, TuiMode::EditReceipts);
}

#[test]
fn tui_mode_inequality() {
    assert_ne!(TuiMode::List, TuiMode::EditTitle);
    assert_ne!(TuiMode::EditTitle, TuiMode::EditSummary);
    assert_ne!(TuiMode::EditSummary, TuiMode::EditReceipts);
    assert_ne!(TuiMode::EditReceipts, TuiMode::List);
}

#[test]
fn tui_mode_clone() {
    let mode = TuiMode::EditTitle;
    let cloned = mode.clone();
    assert_eq!(mode, cloned);
}

#[test]
fn tui_mode_debug() {
    let mode = TuiMode::EditReceipts;
    let debug = format!("{:?}", mode);
    assert!(debug.contains("EditReceipts"));
}

#[test]
fn tui_mode_all_variants() {
    let modes = [
        TuiMode::List,
        TuiMode::EditTitle,
        TuiMode::EditSummary,
        TuiMode::EditReceipts,
    ];
    // All should be distinct
    for (i, m1) in modes.iter().enumerate() {
        for (j, m2) in modes.iter().enumerate() {
            if i == j {
                assert_eq!(m1, m2);
            } else {
                assert_ne!(m1, m2);
            }
        }
    }
}

// ── TuiEditor ─────────────────────────────────────────────────────────

#[test]
fn tui_editor_creates() {
    let config = TuiConfig {
        workstreams_path: PathBuf::from("test.yaml"),
        show_receipts: true,
    };
    let _editor = TuiEditor::new(config);
}

#[test]
fn tui_editor_with_various_configs() {
    let configs = vec![
        TuiConfig {
            workstreams_path: PathBuf::from(""),
            show_receipts: false,
        },
        TuiConfig {
            workstreams_path: PathBuf::from("/absolute/path.yaml"),
            show_receipts: true,
        },
        TuiConfig {
            workstreams_path: PathBuf::from("relative/path.yaml"),
            show_receipts: true,
        },
    ];
    for config in configs {
        let _editor = TuiEditor::new(config);
    }
}
