//! Validation logic for shiplog events and packets.
//!
//! Provides validators for ensuring data integrity and schema compliance
//! across the shiplog pipeline.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Validation error types for shiplog entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "validation error on '{}': {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validator for shiplog events.
pub struct EventValidator;

impl EventValidator {
    /// Validates an event ID format (SHA-256 hex string).
    pub fn validate_event_id(id: &str) -> ValidationResult<()> {
        if id.is_empty() {
            return Err(ValidationError {
                field: "event_id".to_string(),
                message: "event ID cannot be empty".to_string(),
            });
        }
        if id.len() != 64 {
            return Err(ValidationError {
                field: "event_id".to_string(),
                message: format!("event ID must be 64 characters, got {}", id.len()),
            });
        }
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ValidationError {
                field: "event_id".to_string(),
                message: "event ID must be valid hexadecimal".to_string(),
            });
        }
        Ok(())
    }

    /// Validates event source.
    pub fn validate_source(source: &str) -> ValidationResult<()> {
        let valid_sources = ["github", "jira", "linear", "gitlab", "manual", "git"];
        if !valid_sources.contains(&source) {
            return Err(ValidationError {
                field: "source".to_string(),
                message: format!(
                    "invalid source '{}', must be one of: {}",
                    source,
                    valid_sources.join(", ")
                ),
            });
        }
        Ok(())
    }
}

/// Validator for shiplog packets.
pub struct PacketValidator;

impl PacketValidator {
    /// Validates a packet has required fields.
    pub fn validate_packet(packet: &Packet) -> ValidationResult<()> {
        if packet.events.is_empty() {
            return Err(ValidationError {
                field: "events".to_string(),
                message: "packet must contain at least one event".to_string(),
            });
        }
        Ok(())
    }
}

/// A shiplog packet containing events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packet {
    pub id: String,
    pub events: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_valid() {
        let valid_id = "a".repeat(64);
        assert!(EventValidator::validate_event_id(&valid_id).is_ok());
    }

    #[test]
    fn event_id_empty() {
        let result = EventValidator::validate_event_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("empty"));
    }

    #[test]
    fn event_id_wrong_length() {
        let result = EventValidator::validate_event_id("abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("64"));
    }

    #[test]
    fn event_id_invalid_hex() {
        let result = EventValidator::validate_event_id(&"g".repeat(64));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("hexadecimal"));
    }

    #[test]
    fn source_valid() {
        assert!(EventValidator::validate_source("github").is_ok());
        assert!(EventValidator::validate_source("jira").is_ok());
    }

    #[test]
    fn source_invalid() {
        let result = EventValidator::validate_source("invalid_source");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("invalid source"));
    }

    #[test]
    fn packet_valid() {
        let packet = Packet {
            id: "test-packet-1".to_string(),
            events: vec!["event1".to_string(), "event2".to_string()],
        };
        assert!(PacketValidator::validate_packet(&packet).is_ok());
    }

    #[test]
    fn packet_empty_events() {
        let packet = Packet {
            id: "test-packet-2".to_string(),
            events: vec![],
        };
        let result = PacketValidator::validate_packet(&packet);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("at least one event"));
    }

    // --- Edge case tests ---

    #[test]
    fn event_id_all_zeros() {
        let id = "0".repeat(64);
        assert!(EventValidator::validate_event_id(&id).is_ok());
    }

    #[test]
    fn event_id_all_fs() {
        let id = "f".repeat(64);
        assert!(EventValidator::validate_event_id(&id).is_ok());
    }

    #[test]
    fn event_id_uppercase_hex_is_valid() {
        let id = "A".repeat(64);
        assert!(EventValidator::validate_event_id(&id).is_ok());
    }

    #[test]
    fn event_id_mixed_case_hex() {
        // "aAbBcCdDeEfF" is 12 chars, repeat 5 = 60, plus "0123" = 64
        let id = "aAbBcCdDeEfF".repeat(5) + "0123";
        assert_eq!(id.len(), 64);
        assert!(EventValidator::validate_event_id(&id).is_ok());
    }

    #[test]
    fn event_id_63_chars_too_short() {
        let id = "a".repeat(63);
        let result = EventValidator::validate_event_id(&id);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("64"));
    }

    #[test]
    fn event_id_65_chars_too_long() {
        let id = "a".repeat(65);
        let result = EventValidator::validate_event_id(&id);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("64"));
    }

    #[test]
    fn event_id_non_hex_at_end() {
        let mut id = "a".repeat(63);
        id.push('z');
        let result = EventValidator::validate_event_id(&id);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("hexadecimal"));
    }

    #[test]
    fn event_id_whitespace_only() {
        let id = " ".repeat(64);
        let result = EventValidator::validate_event_id(&id);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("hexadecimal"));
    }

    #[test]
    fn source_all_valid_sources() {
        for source in &["github", "jira", "linear", "gitlab", "manual", "git"] {
            assert!(
                EventValidator::validate_source(source).is_ok(),
                "expected '{}' to be valid",
                source
            );
        }
    }

    #[test]
    fn source_empty_string_is_invalid() {
        let result = EventValidator::validate_source("");
        assert!(result.is_err());
    }

    #[test]
    fn source_case_sensitive() {
        assert!(EventValidator::validate_source("GitHub").is_err());
        assert!(EventValidator::validate_source("GITHUB").is_err());
    }

    #[test]
    fn source_with_whitespace_is_invalid() {
        assert!(EventValidator::validate_source(" github").is_err());
        assert!(EventValidator::validate_source("github ").is_err());
    }

    #[test]
    fn packet_single_event_is_valid() {
        let packet = Packet {
            id: "p".to_string(),
            events: vec!["e1".to_string()],
        };
        assert!(PacketValidator::validate_packet(&packet).is_ok());
    }

    #[test]
    fn packet_many_events_is_valid() {
        let packet = Packet {
            id: "p".to_string(),
            events: (0..1000).map(|i| format!("e{}", i)).collect(),
        };
        assert!(PacketValidator::validate_packet(&packet).is_ok());
    }

    #[test]
    fn validation_error_display() {
        let err = ValidationError {
            field: "test_field".to_string(),
            message: "something went wrong".to_string(),
        };
        let displayed = format!("{}", err);
        assert!(displayed.contains("test_field"));
        assert!(displayed.contains("something went wrong"));
    }

    #[test]
    fn validation_error_is_std_error() {
        let err = ValidationError {
            field: "f".to_string(),
            message: "m".to_string(),
        };
        let std_err: &dyn std::error::Error = &err;
        assert!(!std_err.to_string().is_empty());
    }

    #[test]
    fn validation_error_serde_roundtrip() {
        let err = ValidationError {
            field: "event_id".to_string(),
            message: "bad id".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: ValidationError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, err.field);
        assert_eq!(deserialized.message, err.message);
    }

    #[test]
    fn packet_serde_roundtrip() {
        let packet = Packet {
            id: "pkt-1".to_string(),
            events: vec!["e1".to_string(), "e2".to_string()],
        };
        let json = serde_json::to_string(&packet).unwrap();
        let deserialized: Packet = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, packet.id);
        assert_eq!(deserialized.events, packet.events);
    }

    // --- Property tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn hex_string_64() -> impl Strategy<Value = String> {
            proptest::string::string_regex("[0-9a-fA-F]{64}").unwrap()
        }

        proptest! {
            #[test]
            fn valid_hex_ids_always_pass(id in hex_string_64()) {
                prop_assert!(EventValidator::validate_event_id(&id).is_ok());
            }

            #[test]
            fn short_ids_always_fail(id in "[0-9a-f]{1,63}") {
                prop_assert!(EventValidator::validate_event_id(&id).is_err());
            }

            #[test]
            fn long_ids_always_fail(id in "[0-9a-f]{65,128}") {
                prop_assert!(EventValidator::validate_event_id(&id).is_err());
            }

            #[test]
            fn non_hex_64_chars_always_fail(id in "[g-z]{64}") {
                prop_assert!(EventValidator::validate_event_id(&id).is_err());
            }

            #[test]
            fn arbitrary_source_rejected_unless_known(source in "[a-z]{1,20}") {
                let valid = ["github", "jira", "linear", "gitlab", "manual", "git"];
                if valid.contains(&source.as_str()) {
                    prop_assert!(EventValidator::validate_source(&source).is_ok());
                } else {
                    prop_assert!(EventValidator::validate_source(&source).is_err());
                }
            }

            #[test]
            fn nonempty_packet_always_valid(n in 1usize..50) {
                let packet = Packet {
                    id: "p".to_string(),
                    events: (0..n).map(|i| format!("e{}", i)).collect(),
                };
                prop_assert!(PacketValidator::validate_packet(&packet).is_ok());
            }

            #[test]
            fn validation_error_serde_roundtrip_prop(
                field in "[a-z_]{1,30}",
                message in ".{1,100}"
            ) {
                let err = ValidationError { field: field.clone(), message: message.clone() };
                let json = serde_json::to_string(&err).unwrap();
                let de: ValidationError = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(de.field, field);
                prop_assert_eq!(de.message, message);
            }
        }
    }
}
