#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_cache_key::CacheKey;

fuzz_target!(|data: &[u8]| {
    let split_a = data.len() / 3;
    let split_b = (data.len() * 2) / 3;

    let query = String::from_utf8_lossy(&data[..split_a]);
    let url = String::from_utf8_lossy(&data[split_a..split_b]);
    let tail = &data[split_b..];

    let page = u32::from(*tail.first().unwrap_or(&0));
    let per_page = u32::from(*tail.get(1).unwrap_or(&1)).max(1);
    let project_id = u64::from(*tail.get(2).unwrap_or(&0));
    let mr_iid = u64::from(*tail.get(3).unwrap_or(&0));

    let search = CacheKey::search(&query, page, per_page);
    let details = CacheKey::pr_details(&url);
    let reviews = CacheKey::pr_reviews(&url, page);
    let notes = CacheKey::mr_notes(project_id, mr_iid, page);

    assert!(search.starts_with("search:"));
    assert!(details.starts_with("pr:details:"));
    assert!(reviews.starts_with("pr:reviews:"));
    assert!(notes.starts_with("gitlab:mr:notes:"));
});
