use shiplog_plugin::{PluginManager, PluginManifest, PluginStatus};
use std::path::PathBuf;

// --- PluginManager integration tests ---

#[test]
fn manager_new_creates_empty_manager() {
    let mgr = PluginManager::new(PathBuf::from("/tmp/plugins"));
    // Manager should be constructable without side effects
    drop(mgr);
}

#[test]
fn manager_with_different_paths() {
    let paths = vec![
        PathBuf::from("/tmp/plugins"),
        PathBuf::from("./plugins"),
        PathBuf::from(""),
    ];
    for p in paths {
        let mgr = PluginManager::new(p);
        drop(mgr);
    }
}

// --- PluginManifest tests ---

#[test]
fn manifest_roundtrip_json() {
    let manifest = PluginManifest {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "A test plugin".to_string(),
        shiplog_version: "0.2.1".to_string(),
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "test-plugin");
    assert_eq!(deserialized.version, "1.0.0");
    assert_eq!(deserialized.description, "A test plugin");
    assert_eq!(deserialized.shiplog_version, "0.2.1");
}

#[test]
fn manifest_clone_is_independent() {
    let manifest = PluginManifest {
        name: "original".to_string(),
        version: "1.0.0".to_string(),
        description: "desc".to_string(),
        shiplog_version: "0.2.1".to_string(),
    };
    let mut cloned = manifest.clone();
    cloned.name = "cloned".to_string();
    assert_eq!(manifest.name, "original");
    assert_eq!(cloned.name, "cloned");
}

// --- PluginStatus tests ---

#[test]
fn status_equality() {
    assert_eq!(PluginStatus::Installed, PluginStatus::Installed);
    assert_eq!(PluginStatus::Enabled, PluginStatus::Enabled);
    assert_eq!(PluginStatus::Disabled, PluginStatus::Disabled);
    assert_ne!(PluginStatus::Installed, PluginStatus::Enabled);
    assert_ne!(PluginStatus::Enabled, PluginStatus::Disabled);
}

#[test]
fn status_roundtrip_json() {
    let statuses = vec![
        PluginStatus::Installed,
        PluginStatus::Enabled,
        PluginStatus::Disabled,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: PluginStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}

#[test]
fn status_debug_format() {
    let s = format!("{:?}", PluginStatus::Installed);
    assert!(s.contains("Installed"));
}

// --- Edge case: empty/special strings ---

#[test]
fn manifest_with_empty_strings() {
    let manifest = PluginManifest {
        name: String::new(),
        version: String::new(),
        description: String::new(),
        shiplog_version: String::new(),
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "");
}

#[test]
fn manifest_with_unicode() {
    let manifest = PluginManifest {
        name: "プラグイン".to_string(),
        version: "1.0.0-α".to_string(),
        description: "描述 🚀".to_string(),
        shiplog_version: "0.2.1".to_string(),
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "プラグイン");
    assert_eq!(deserialized.description, "描述 🚀");
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn manifest_json_roundtrip(
            name in "\\PC{1,50}",
            version in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
            desc in "\\PC{0,100}",
        ) {
            let manifest = PluginManifest {
                name: name.clone(),
                version: version.clone(),
                description: desc.clone(),
                shiplog_version: "0.2.1".to_string(),
            };
            let json = serde_json::to_string(&manifest).unwrap();
            let back: PluginManifest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(&back.name, &name);
            prop_assert_eq!(&back.version, &version);
            prop_assert_eq!(&back.description, &desc);
        }

        #[test]
        fn status_clone_equals_original(idx in 0u8..3) {
            let status = match idx {
                0 => PluginStatus::Installed,
                1 => PluginStatus::Enabled,
                _ => PluginStatus::Disabled,
            };
            let cloned = status.clone();
            prop_assert_eq!(status, cloned);
        }
    }
}
