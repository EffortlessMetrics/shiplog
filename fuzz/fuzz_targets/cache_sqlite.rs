#![no_main]

use chrono::Duration;
use libfuzzer_sys::fuzz_target;
use shiplog_cache_sqlite::ApiCache;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let cache = ApiCache::open_in_memory().expect("open in-memory cache");
    let action = data[0] % 5;
    let key = format!("key-{}", data.get(1).copied().unwrap_or(0));
    let value = String::from_utf8_lossy(data).into_owned();
    let ttl = Duration::seconds(data.get(2).copied().unwrap_or(0) as i64 - 128);

    match action {
        0 => {
            let _ = cache.set(&key, &value);
            let _ = cache.get::<String>(&key);
            let _ = cache.contains(&key);
            let stats = cache.stats().unwrap();
            assert!(stats.expired_entries <= stats.total_entries);
        }
        1 => {
            let _ = cache.set_with_ttl(&key, &value, ttl);
            let _ = cache.get::<String>(&key);
            let _ = cache.cleanup_expired();
            let stats = cache.stats().unwrap();
            assert!(stats.total_entries >= stats.valid_entries);
        }
        2 => {
            let _ = cache.contains(&key);
            let _ = cache.clear();
            let _ = cache.cleanup_expired();
        }
        3 => {
            let _ = cache.set_with_ttl(&key, &value, Duration::seconds(90));
            let reopened = ApiCache::open_in_memory().unwrap();
            let reopened_stats = reopened.stats().unwrap();
            assert_eq!(reopened_stats.total_entries, 0);
        }
        _ => {
            let _ = cache.set(&key, &value);
            let _ = cache.stats();
            let _ = cache.clear();
        }
    }
});
