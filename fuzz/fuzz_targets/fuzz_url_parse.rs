//! Fuzz harness for URL parsing (shiplog-url)
//!
//! Exercises `parse_url`, `is_valid_url`, and accessor helpers with arbitrary
//! strings.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_url::{get_host, get_path, get_scheme, is_valid_url, parse_url};

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let _ = is_valid_url(input);

    if let Ok(url) = parse_url(input) {
        let _ = get_host(&url);
        let _ = get_path(&url);
        let _ = get_scheme(&url);
    }
});
