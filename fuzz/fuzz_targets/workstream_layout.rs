//! Fuzz harness for workstream artifact layout.
//! 
//! This target covers YAML parsing and workstream file load/write behavior.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_workstream_layout::{CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, write_workstreams};
use shiplog_schema::workstream::WorkstreamsFile;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let parsed: WorkstreamsFile = match serde_yaml::from_str(input) {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let mut dir = std::env::temp_dir();
    dir.push("shiplog-fuzz-workstream-layout");
    let _ = std::fs::create_dir_all(&dir);
    let suggested = dir.join(SUGGESTED_FILENAME);
    let curated = dir.join(CURATED_FILENAME);

    let _ = write_workstreams(&suggested, &parsed);
    let _ = WorkstreamManager::try_load(&dir);
    let _ = write_workstreams(&curated, &parsed);
    let _ = WorkstreamManager::try_load(&dir);
});
