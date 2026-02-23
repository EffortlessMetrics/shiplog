//! Fuzz harness for profile-based redaction policy.
//!
//! This harness parses JSON-encoded events/workstreams and applies the
//! extracted redaction policy across profiles.

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;
use shiplog_redaction_policy::{
    RedactionProfile, redact_events_with_aliases, redact_workstreams_with_aliases,
};
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;

#[derive(Debug, Deserialize)]
struct FuzzInput {
    events: Vec<EventEnvelope>,
    workstreams: Option<WorkstreamsFile>,
    profile: Option<String>,
}

fn alias(kind: &str, value: &str) -> String {
    let mut acc = 14695981039346656037u64;
    for byte in kind.bytes().chain(value.bytes()) {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(1099511628211);
    }
    format!("{kind}-{acc:016x}")
}

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(text) => text,
        Err(_) => return,
    };

    let parsed: FuzzInput = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(_) => return,
    };

    let profile = RedactionProfile::from_profile_str(
        parsed
            .profile
            .as_deref()
            .unwrap_or("public"),
    );

    let _ = redact_events_with_aliases(&parsed.events, profile, &alias);

    if let Some(workstreams) = parsed.workstreams {
        let _ = redact_workstreams_with_aliases(&workstreams, profile, &alias);
    }
});
