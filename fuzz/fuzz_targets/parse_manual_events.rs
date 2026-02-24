//! Fuzz harness for Manual Events YAML (manual_events.yaml)
//!
//! This harness tests the robustness of the manual events parser against
//! malformed input.
//! Target: ManualEventsFile YAML

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_manual_events::events_in_window;
use shiplog_schema::event::ManualEventsFile;

fuzz_target!(|data: &[u8]| {
    // Ensure the input is valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip non-UTF-8 input
    };

    let Ok(file) = serde_yaml::from_str::<ManualEventsFile>(input) else {
        return;
    };
    let window = shiplog_schema::coverage::TimeWindow {
        since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };
    let _ = events_in_window(&file.events, "fuzzer", &window);
});
