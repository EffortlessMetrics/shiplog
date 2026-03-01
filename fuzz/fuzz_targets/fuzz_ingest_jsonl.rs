//! Fuzz harness for JSONL event parsing (shiplog-ingest-json)
//!
//! Feeds arbitrary text into the JSONL parser, exercising multi-line splitting
//! and per-line JSON deserialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_ingest_json::parse_events_jsonl;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let _ = parse_events_jsonl(input, "fuzz");
});
