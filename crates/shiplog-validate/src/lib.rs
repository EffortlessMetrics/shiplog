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
}
