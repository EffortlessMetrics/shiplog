//! Fuzz harness for redaction profile parsing.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_redaction_profile::RedactionProfile;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(text) => text,
        Err(_) => return,
    };

    let parsed = RedactionProfile::from_profile_str(input);
    let canonical = parsed.as_str();

    // Canonical rendering always reparses to the same profile.
    let reparsed = RedactionProfile::from_profile_str(canonical);
    assert_eq!(parsed, reparsed);

    // `FromStr` is intentionally infallible and must match helper parsing.
    let parsed_from_str: RedactionProfile = input.parse().expect("infallible parse");
    assert_eq!(parsed, parsed_from_str);
});
