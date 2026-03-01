//! Fuzz harness for team YAML config parsing (shiplog-team-core)
//!
//! Deserializes arbitrary YAML into `TeamConfig` and exercises helper methods.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_team_core::TeamConfig;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    if let Ok(cfg) = serde_yaml::from_str::<TeamConfig>(input) {
        let _ = cfg.normalized_sections();
        let _ = cfg.section_enabled("summary");
    }
});
