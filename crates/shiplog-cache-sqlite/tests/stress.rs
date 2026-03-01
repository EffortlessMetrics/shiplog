//! Multi-threaded stress tests for concurrent SQLite cache access.

use shiplog_cache_sqlite::ApiCache;

#[test]
fn concurrent_writes_no_panic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("stress.db");

    // Initialise the schema before spawning threads.
    drop(ApiCache::open(&db_path).unwrap());

    std::thread::scope(|s| {
        for t in 0..8u32 {
            let path = db_path.clone();
            s.spawn(move || {
                let cache = ApiCache::open(&path).unwrap();
                for i in 0..50u32 {
                    let key = format!("t{t}:k{i}");
                    let val = format!("v{t}_{i}");
                    // Writes may fail with SQLITE_BUSY under contention; no panics.
                    let _ = cache.set(&key, &val);
                    let _ = cache.get::<String>(&key);
                    let _ = cache.contains(&key);
                }
            });
        }
    });

    // Verify the database is not corrupted.
    let cache = ApiCache::open(&db_path).unwrap();
    let stats = cache.stats().unwrap();
    assert!(
        stats.total_entries > 0,
        "expected at least some entries to survive concurrent writes"
    );
}

#[test]
fn concurrent_reads_consistent() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("stress_read.db");

    // Pre-populate the database from the main thread.
    let cache = ApiCache::open(&db_path).unwrap();
    for i in 0..100u32 {
        cache.set(&format!("key{i}"), &format!("value{i}")).unwrap();
    }
    drop(cache);

    // Concurrent reads should all succeed and return consistent data.
    std::thread::scope(|s| {
        for _ in 0..8 {
            let path = db_path.clone();
            s.spawn(move || {
                let cache = ApiCache::open(&path).unwrap();
                for i in 0..100u32 {
                    let val: Option<String> = cache.get(&format!("key{i}")).unwrap();
                    assert_eq!(
                        val.as_deref(),
                        Some(format!("value{i}")).as_deref(),
                        "reader thread got wrong value for key{i}"
                    );
                }
            });
        }
    });
}

#[test]
fn concurrent_mixed_operations_no_corruption() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("stress_mixed.db");

    drop(ApiCache::open(&db_path).unwrap());

    std::thread::scope(|s| {
        // Writer threads
        for t in 0..4u32 {
            let path = db_path.clone();
            s.spawn(move || {
                let cache = ApiCache::open(&path).unwrap();
                for i in 0..30u32 {
                    let _ = cache.set(&format!("w{t}:k{i}"), &i);
                }
            });
        }
        // Reader / cleanup threads
        for _ in 0..4 {
            let path = db_path.clone();
            s.spawn(move || {
                let cache = ApiCache::open(&path).unwrap();
                for _ in 0..30 {
                    let _ = cache.stats();
                    let _ = cache.cleanup_expired();
                    let _ = cache.contains("w0:k0");
                }
            });
        }
    });

    // Final integrity check.
    let cache = ApiCache::open(&db_path).unwrap();
    assert!(cache.stats().is_ok(), "database should remain queryable");
}
