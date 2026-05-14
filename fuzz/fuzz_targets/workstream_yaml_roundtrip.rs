//! Fuzz harness for workstream YAML round-trip verification.
//!
//! Goes beyond basic parsing: if a `WorkstreamsFile` successfully deserializes
//! from fuzzed YAML, it is serialized back and re-parsed to verify consistency.
//! Also exercises `Workstream` stat increments and event-ID handling.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog::schema::event::EventKind;
use shiplog::schema::workstream::WorkstreamsFile;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Phase 1: parse arbitrary YAML — must not panic
    let parsed: WorkstreamsFile = match serde_yaml::from_str(input) {
        Ok(wf) => wf,
        Err(_) => return,
    };

    // Phase 2: round-trip — serialize back to YAML and re-parse
    let yaml_out = match serde_yaml::to_string(&parsed) {
        Ok(s) => s,
        Err(_) => return,
    };
    let reparsed: WorkstreamsFile = match serde_yaml::from_str(&yaml_out) {
        Ok(wf) => wf,
        Err(_) => return,
    };

    // Invariant: workstream count preserved through round-trip
    assert_eq!(
        parsed.workstreams.len(),
        reparsed.workstreams.len(),
        "workstream count must survive YAML round-trip"
    );

    // Phase 3: exercise Workstream::bump_stats on each workstream
    for ws in &parsed.workstreams {
        if let Some(expected) = ws.stats.pull_requests.checked_add(1) {
            let mut cloned = ws.clone();
            cloned.bump_stats(&EventKind::PullRequest);
            assert_eq!(cloned.stats.pull_requests, expected);
        }

        if let Some(expected) = ws.stats.reviews.checked_add(1) {
            let mut cloned = ws.clone();
            cloned.bump_stats(&EventKind::Review);
            assert_eq!(cloned.stats.reviews, expected);
        }

        if let Some(expected) = ws.stats.manual_events.checked_add(1) {
            let mut cloned = ws.clone();
            cloned.bump_stats(&EventKind::Manual);
            assert_eq!(cloned.stats.manual_events, expected);
        }
    }

    // Phase 4: JSON round-trip of the same data
    if let Ok(json) = serde_json::to_string(&parsed) {
        let _ = serde_json::from_str::<WorkstreamsFile>(&json);
    }
});
