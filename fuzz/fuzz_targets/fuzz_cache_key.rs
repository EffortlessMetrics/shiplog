//! Fuzz harness for cache key generation (shiplog-cache-key)
//!
//! Feeds arbitrary strings and integers into every `CacheKey` constructor,
//! asserting that the expected prefix is always present.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_cache_key::CacheKey;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let split = data.len() / 2;
    let query = String::from_utf8_lossy(&data[..split]);
    let url = String::from_utf8_lossy(&data[split..]);

    let page = u32::from_le_bytes([
        *data.first().unwrap_or(&0),
        *data.get(1).unwrap_or(&0),
        *data.get(2).unwrap_or(&0),
        *data.get(3).unwrap_or(&0),
    ]);
    let per_page = u32::from_le_bytes([
        *data.get(4).unwrap_or(&1),
        *data.get(5).unwrap_or(&0),
        *data.get(6).unwrap_or(&0),
        *data.get(7).unwrap_or(&0),
    ])
    .max(1);

    let project_id = u64::from(page);
    let mr_iid = u64::from(per_page);

    let search = CacheKey::search(&query, page, per_page);
    let details = CacheKey::pr_details(&url);
    let reviews = CacheKey::pr_reviews(&url, page);
    let notes = CacheKey::mr_notes(project_id, mr_iid, page);

    assert!(search.starts_with("search:"));
    assert!(details.starts_with("pr:details:"));
    assert!(reviews.starts_with("pr:reviews:"));
    assert!(notes.starts_with("gitlab:mr:notes:"));
});
