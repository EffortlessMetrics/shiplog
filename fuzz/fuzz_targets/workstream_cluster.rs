#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::event::EventEnvelope;
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_ports::WorkstreamClusterer;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Accept JSON input only: arrays of EventEnvelope from any source.
    let events: Vec<EventEnvelope> = match serde_json::from_str(input) {
        Ok(events) => events,
        Err(_) => return,
    };

    let _ = RepoClusterer.cluster(&events);
});
