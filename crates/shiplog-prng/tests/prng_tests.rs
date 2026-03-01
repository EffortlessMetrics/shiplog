use shiplog_prng::*;

#[test]
fn seeded_rng_deterministic() {
    let mut r1 = SeededRng::new(42);
    let mut r2 = SeededRng::new(42);
    let vals1: Vec<i32> = (0..10).map(|_| r1.range(0, 1000)).collect();
    let vals2: Vec<i32> = (0..10).map(|_| r2.range(0, 1000)).collect();
    assert_eq!(vals1, vals2);
}

#[test]
fn seeded_rng_range_in_bounds() {
    let mut rng = SeededRng::new(123);
    for _ in 0..100 {
        let v = rng.range(10, 20);
        assert!((10..=20).contains(&v));
    }
}

#[test]
fn seeded_rng_float_in_range() {
    let mut rng = SeededRng::new(42);
    for _ in 0..100 {
        let f = rng.float();
        assert!((0.0..1.0).contains(&f));
    }
}

#[test]
fn seeded_rng_bytes_length() {
    let mut rng = SeededRng::new(42);
    assert_eq!(rng.bytes(0).len(), 0);
    assert_eq!(rng.bytes(16).len(), 16);
    assert_eq!(rng.bytes(256).len(), 256);
}

#[test]
fn seeded_rng_string_length() {
    let mut rng = SeededRng::new(42);
    assert_eq!(rng.string(10).len(), 10);
    assert_eq!(rng.string(0).len(), 0);
}

#[test]
fn seeded_rng_pick_from_slice() {
    let mut rng = SeededRng::new(42);
    let items = [1, 2, 3, 4, 5];
    let picked = rng.pick(&items);
    assert!(picked.is_some());
    assert!(items.contains(picked.unwrap()));
}

#[test]
fn seeded_rng_pick_empty() {
    let mut rng = SeededRng::new(42);
    let empty: [i32; 0] = [];
    assert!(rng.pick(&empty).is_none());
}

#[test]
fn seeded_rng_reset_reproduces() {
    let mut rng = SeededRng::new(42);
    let v1 = rng.range(0, 100);
    rng.reset(42);
    let v2 = rng.range(0, 100);
    assert_eq!(v1, v2);
}

#[test]
fn deterministic_seq_reproducible() {
    let s1 = deterministic_seq(42, 10);
    let s2 = deterministic_seq(42, 10);
    assert_eq!(s1, s2);
    assert_eq!(s1.len(), 10);
}

#[test]
fn deterministic_string_reproducible() {
    let s1 = deterministic_string(42, 20);
    let s2 = deterministic_string(42, 20);
    assert_eq!(s1, s2);
    assert_eq!(s1.len(), 20);
}

#[test]
fn deterministic_bytes_reproducible() {
    let b1 = deterministic_bytes(42, 16);
    let b2 = deterministic_bytes(42, 16);
    assert_eq!(b1, b2);
}

#[test]
fn lcg_deterministic() {
    let mut l1 = Lcg::new(42);
    let mut l2 = Lcg::new(42);
    for _ in 0..10 {
        assert_eq!(l1.next_val(), l2.next_val());
    }
}

#[test]
fn lcg_range_in_bounds() {
    let mut lcg = Lcg::new(42);
    for _ in 0..100 {
        let v = lcg.range(1, 100);
        assert!((1..=100).contains(&v));
    }
}

#[test]
fn lcg_float_in_range() {
    let mut lcg = Lcg::new(42);
    for _ in 0..100 {
        let f = lcg.float();
        assert!((0.0..1.0).contains(&f));
    }
}

#[test]
fn lcg_default_seed() {
    let mut d = Lcg::default();
    let mut s = Lcg::new(12345);
    assert_eq!(d.next_val(), s.next_val());
}

#[test]
fn lcg_reset_reproduces() {
    let mut lcg = Lcg::new(42);
    let v1 = lcg.next_val();
    lcg.reset(42);
    let v2 = lcg.next_val();
    assert_eq!(v1, v2);
}
