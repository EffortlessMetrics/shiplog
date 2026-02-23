//! Fuzz harness for JSONL ingestion (ledger.events.jsonl)
//!
//! This harness tests the robustness of the JSONL parser against malformed input.
//! Target: `shiplog_ingest_json` line parser

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_ingest_json::parse_events_jsonl;

fuzz_target!(|data: &[u8]| {
    // Ensure the input is valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip non-UTF-8 input
    };

    // Parse the input as JSONL — we don't care if it fails,
    // we just want to ensure it doesn't panic
    let _ = parse_events_jsonl(input, "fuzz");
});
