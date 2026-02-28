use shiplog_validate::*;

#[test]
fn valid_event_id() {
    let id = "a".repeat(64);
    assert!(EventValidator::validate_event_id(&id).is_ok());
}

#[test]
fn event_id_empty() {
    let err = EventValidator::validate_event_id("").unwrap_err();
    assert!(err.message.contains("empty"));
}

#[test]
fn event_id_wrong_length() {
    assert!(EventValidator::validate_event_id(&"a".repeat(63)).is_err());
    assert!(EventValidator::validate_event_id(&"a".repeat(65)).is_err());
}

#[test]
fn event_id_non_hex() {
    assert!(EventValidator::validate_event_id(&"g".repeat(64)).is_err());
}

#[test]
fn event_id_mixed_case_hex() {
    let id = "aAbBcCdDeEfF".repeat(5) + "0123";
    assert_eq!(id.len(), 64);
    assert!(EventValidator::validate_event_id(&id).is_ok());
}

#[test]
fn valid_sources() {
    for s in &["github", "jira", "linear", "gitlab", "manual", "git"] {
        assert!(
            EventValidator::validate_source(s).is_ok(),
            "expected '{}' valid",
            s
        );
    }
}

#[test]
fn invalid_source() {
    assert!(EventValidator::validate_source("unknown").is_err());
    assert!(EventValidator::validate_source("").is_err());
    assert!(EventValidator::validate_source("GitHub").is_err());
}

#[test]
fn packet_valid() {
    let packet = Packet {
        id: "p1".to_string(),
        events: vec!["e1".to_string()],
    };
    assert!(PacketValidator::validate_packet(&packet).is_ok());
}

#[test]
fn packet_empty_events() {
    let packet = Packet {
        id: "p1".to_string(),
        events: vec![],
    };
    assert!(PacketValidator::validate_packet(&packet).is_err());
}

#[test]
fn validation_error_display() {
    let err = ValidationError {
        field: "f".to_string(),
        message: "m".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("f"));
    assert!(s.contains("m"));
}

#[test]
fn packet_serde_roundtrip() {
    let packet = Packet {
        id: "p1".to_string(),
        events: vec!["e1".to_string(), "e2".to_string()],
    };
    let json = serde_json::to_string(&packet).unwrap();
    let de: Packet = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, packet.id);
    assert_eq!(de.events, packet.events);
}
