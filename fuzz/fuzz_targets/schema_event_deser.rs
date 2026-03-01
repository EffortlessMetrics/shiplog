//! Fuzz harness for EventEnvelope JSON deserialization.
//!
//! This is a critical parsing surface — events arrive from GitHub API
//! responses, JSONL imports, and manual YAML files. Malformed data must
//! never cause panics.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::event::EventEnvelope;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Fuzz single event deserialization
    if let Ok(event) = serde_json::from_str::<EventEnvelope>(input) {
        // Round-trip: serialize back and verify no panic
        let _ = serde_json::to_string(&event);
    }

    // Also fuzz as a list of events (JSONL-adjacent)
    let _ = serde_json::from_str::<Vec<EventEnvelope>>(input);
});
