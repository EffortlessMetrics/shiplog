//! Fuzz harness for YAML workstream config parsing (shiplog-workstreams)
//!
//! Exercises the `WorkstreamsFile` deserializer with arbitrary YAML input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::workstream::WorkstreamsFile;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let _: Result<WorkstreamsFile, _> = serde_yaml::from_str(input);
});
