//! Fuzz harness for deterministic event redaction (security-critical).
//!
//! Exercises the DeterministicRedactor with arbitrary JSON-encoded events
//! and keys across all redaction profiles, ensuring no panics occur.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_redact::DeterministicRedactor;
use shiplog_schema::event::EventEnvelope;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    // Use first byte to select profile, second byte as key seed
    let profile_selector = data[0] % 3;
    let profile = match profile_selector {
        0 => "internal",
        1 => "manager",
        _ => "public",
    };

    // Use second byte onward as the redaction key
    let key = &data[1..data.len().min(33)];
    let json_start = data.len().min(33);
    if json_start >= data.len() {
        return;
    }

    let json_bytes = &data[json_start..];
    let input = match std::str::from_utf8(json_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    let events: Vec<EventEnvelope> = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => {
            // Also try parsing as a single event
            match serde_json::from_str::<EventEnvelope>(input) {
                Ok(ev) => vec![ev],
                Err(_) => return,
            }
        }
    };

    let redactor = DeterministicRedactor::new(key);
    let _ = redactor.redact_events(&events, profile);
});
