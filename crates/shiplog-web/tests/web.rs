use shiplog_web::*;
use std::path::PathBuf;

// ── WebConfig ─────────────────────────────────────────────────────────

#[test]
fn web_config_fields() {
    let config = WebConfig {
        packet_path: PathBuf::from("packet.md"),
        host: "0.0.0.0".to_string(),
        port: 3000,
    };
    assert_eq!(config.packet_path, PathBuf::from("packet.md"));
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
}

#[test]
fn web_config_clone() {
    let config = WebConfig {
        packet_path: PathBuf::from("test.md"),
        host: "localhost".to_string(),
        port: 9090,
    };
    let cloned = config.clone();
    assert_eq!(cloned.packet_path, PathBuf::from("test.md"));
    assert_eq!(cloned.host, "localhost");
    assert_eq!(cloned.port, 9090);
}

#[test]
fn web_config_serde_roundtrip() {
    let config = WebConfig {
        packet_path: PathBuf::from("output/packet.md"),
        host: "192.168.1.1".to_string(),
        port: 4000,
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: WebConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.packet_path, PathBuf::from("output/packet.md"));
    assert_eq!(deserialized.host, "192.168.1.1");
    assert_eq!(deserialized.port, 4000);
}

#[test]
fn web_config_serde_defaults() {
    let json = r#"{}"#;
    let config: WebConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.packet_path, PathBuf::default());
    assert_eq!(config.host, "127.0.0.1"); // default_host
    assert_eq!(config.port, 8080); // default_port
}

#[test]
fn web_config_serde_partial_defaults() {
    let json = r#"{"port": 3000}"#;
    let config: WebConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.host, "127.0.0.1"); // default
    assert_eq!(config.port, 3000); // overridden
}

#[test]
fn web_config_debug() {
    let config = WebConfig {
        packet_path: PathBuf::from("p.md"),
        host: "host".to_string(),
        port: 1234,
    };
    let debug = format!("{:?}", config);
    assert!(debug.contains("p.md"));
    assert!(debug.contains("host"));
    assert!(debug.contains("1234"));
}

// ── WebViewer ─────────────────────────────────────────────────────────

#[test]
fn web_viewer_creates() {
    let config = WebConfig {
        packet_path: PathBuf::from("packet.md"),
        host: "127.0.0.1".to_string(),
        port: 8080,
    };
    let _viewer = WebViewer::new(config);
}

#[test]
fn web_viewer_various_configs() {
    let configs = vec![
        WebConfig {
            packet_path: PathBuf::from(""),
            host: "".to_string(),
            port: 0,
        },
        WebConfig {
            packet_path: PathBuf::from("/absolute/packet.md"),
            host: "::1".to_string(),
            port: 65535,
        },
        WebConfig {
            packet_path: PathBuf::from("relative/path/packet.md"),
            host: "127.0.0.1".to_string(),
            port: 443,
        },
    ];
    for config in configs {
        let _viewer = WebViewer::new(config);
    }
}

// ── Port edge cases ───────────────────────────────────────────────────

#[test]
fn web_config_port_zero() {
    let config = WebConfig {
        packet_path: PathBuf::from("p.md"),
        host: "localhost".to_string(),
        port: 0,
    };
    assert_eq!(config.port, 0);
}

#[test]
fn web_config_port_max() {
    let config = WebConfig {
        packet_path: PathBuf::from("p.md"),
        host: "localhost".to_string(),
        port: u16::MAX,
    };
    assert_eq!(config.port, 65535);
}
