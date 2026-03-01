use proptest::prelude::*;
use shiplog_pool::*;
use std::sync::Arc;
use std::thread;

// ── PoolConfig & PoolBuilder ────────────────────────────────────────

#[test]
fn pool_config_default_values() {
    let c = PoolConfig::default();
    assert_eq!(c.max_size, 10);
    assert_eq!(c.min_size, 2);
    assert!(!c.preallocate);
}

#[test]
fn builder_default_matches_new() {
    let a = PoolBuilder::default().build();
    let b = PoolBuilder::new().build();
    assert_eq!(a.max_size, b.max_size);
    assert_eq!(a.min_size, b.min_size);
    assert_eq!(a.preallocate, b.preallocate);
}

proptest! {
    #[test]
    fn builder_preserves_max_size(size in 1usize..1000) {
        let c = PoolBuilder::new().max_size(size).build();
        prop_assert_eq!(c.max_size, size);
    }

    #[test]
    fn builder_preserves_min_size(size in 0usize..100) {
        let c = PoolBuilder::new().min_size(size).build();
        prop_assert_eq!(c.min_size, size);
    }
}

// ── ObjectPool basics ───────────────────────────────────────────────

#[test]
fn empty_pool_get_returns_none() {
    let pool: ObjectPool<String> = ObjectPool::new(5);
    assert_eq!(pool.get(), None);
    assert!(pool.is_empty());
    assert_eq!(pool.len(), 0);
}

#[test]
fn put_then_get_returns_item() {
    let pool: ObjectPool<String> = ObjectPool::new(5);
    assert!(pool.put("hello".into()));
    assert_eq!(pool.len(), 1);
    let item = pool.get();
    assert_eq!(item.as_deref(), Some("hello"));
    assert!(pool.is_empty());
}

#[test]
fn fifo_ordering() {
    let pool: ObjectPool<i32> = ObjectPool::new(10);
    pool.put(1);
    pool.put(2);
    pool.put(3);
    assert_eq!(pool.get(), Some(1));
    assert_eq!(pool.get(), Some(2));
    assert_eq!(pool.get(), Some(3));
    assert_eq!(pool.get(), None);
}

#[test]
fn put_rejects_when_full() {
    let pool: ObjectPool<i32> = ObjectPool::new(2);
    assert!(pool.put(1));
    assert!(pool.put(2));
    assert!(!pool.put(3));
    assert_eq!(pool.len(), 2);
}

#[test]
fn put_after_get_creates_space() {
    let pool: ObjectPool<i32> = ObjectPool::new(1);
    assert!(pool.put(10));
    assert!(!pool.put(20));
    let _ = pool.get();
    assert!(pool.put(30));
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn zero_capacity_pool() {
    let pool: ObjectPool<i32> = ObjectPool::new(0);
    assert!(!pool.put(1));
    assert_eq!(pool.get(), None);
    assert!(pool.is_empty());
}

proptest! {
    #[test]
    fn put_up_to_max_size(max in 1usize..50) {
        let pool: ObjectPool<i32> = ObjectPool::new(max);
        for i in 0..max {
            prop_assert!(pool.put(i as i32));
        }
        prop_assert!(!pool.put(999));
        prop_assert_eq!(pool.len(), max);
    }

    #[test]
    fn get_returns_all_put_items(items in prop::collection::vec(0i32..1000, 1..30)) {
        let pool: ObjectPool<i32> = ObjectPool::new(items.len() + 1);
        for &item in &items {
            pool.put(item);
        }
        let mut retrieved = Vec::new();
        while let Some(v) = pool.get() {
            retrieved.push(v);
        }
        prop_assert_eq!(retrieved, items);
    }
}

// ── Concurrency ─────────────────────────────────────────────────────

#[test]
fn concurrent_put_and_get() {
    let pool = Arc::new(ObjectPool::<i32>::new(100));
    let mut handles = Vec::new();

    // Spawn writers
    for i in 0..10 {
        let p = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            for j in 0..10 {
                let _ = p.put(i * 10 + j);
            }
        }));
    }

    // Spawn readers
    for _ in 0..5 {
        let p = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            for _ in 0..20 {
                let _ = p.get();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    // No panic / deadlock = success
}

// ── Pooled auto-return ──────────────────────────────────────────────

#[test]
fn pooled_get_and_get_mut() {
    use std::sync::LazyLock;
    static POOL: LazyLock<ObjectPool<Vec<u8>>> = LazyLock::new(|| ObjectPool::new(5));

    let mut p = Pooled::new(&POOL, vec![1, 2, 3]);
    assert_eq!(p.get(), &vec![1, 2, 3]);
    p.get_mut().push(4);
    assert_eq!(p.get(), &vec![1, 2, 3, 4]);
}

#[test]
fn pooled_returns_on_drop() {
    use std::sync::LazyLock;
    static POOL2: LazyLock<ObjectPool<String>> = LazyLock::new(|| ObjectPool::new(5));

    assert_eq!(POOL2.len(), 0);
    {
        let _p = Pooled::new(&POOL2, "returned".to_string());
    }
    assert_eq!(POOL2.len(), 1);
    let returned = POOL2.get().unwrap();
    assert_eq!(returned, "returned");
}
