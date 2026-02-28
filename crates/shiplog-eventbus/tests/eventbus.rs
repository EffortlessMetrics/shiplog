use shiplog_eventbus::{Event, EventBus};
use std::sync::{Arc, RwLock};

// --- Event tests ---

#[test]
fn event_new_has_fields() {
    let e = Event::new("user.created", "payload".into(), "auth");
    assert_eq!(e.event_type, "user.created");
    assert_eq!(e.payload, "payload");
    assert_eq!(e.source, "auth");
    assert!(!e.id.is_empty());
}

#[test]
fn event_with_payload_serializes() {
    let e = Event::with_payload("test", &serde_json::json!({"n": 42}), "src");
    assert!(e.payload.contains("42"));
}

// --- EventBus subscribe/emit tests ---

#[test]
fn bus_subscribe_and_emit() {
    let bus = EventBus::new(100);
    let received = Arc::new(RwLock::new(Vec::new()));
    let r = received.clone();

    bus.subscribe("evt", move |e| {
        r.write().unwrap().push(e.payload.clone());
    });
    bus.emit_simple("evt", "data".into(), "src");

    let msgs = received.read().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], "data");
}

#[test]
fn bus_emit_no_handlers() {
    let bus = EventBus::new(10);
    // Should not panic
    bus.emit_simple("no-handler", "data".into(), "src");
}

#[test]
fn bus_multiple_handlers_same_event() {
    let bus = EventBus::new(100);
    let count = Arc::new(RwLock::new(0));

    for _ in 0..3 {
        let c = count.clone();
        bus.subscribe("evt", move |_| {
            *c.write().unwrap() += 1;
        });
    }

    bus.emit_simple("evt", "data".into(), "src");
    assert_eq!(*count.read().unwrap(), 3);
}

#[test]
fn bus_different_event_types_are_independent() {
    let bus = EventBus::new(100);
    let received_a = Arc::new(RwLock::new(0));
    let received_b = Arc::new(RwLock::new(0));

    let ra = received_a.clone();
    bus.subscribe("a", move |_| {
        *ra.write().unwrap() += 1;
    });

    let rb = received_b.clone();
    bus.subscribe("b", move |_| {
        *rb.write().unwrap() += 1;
    });

    bus.emit_simple("a", "".into(), "src");
    bus.emit_simple("a", "".into(), "src");
    bus.emit_simple("b", "".into(), "src");

    assert_eq!(*received_a.read().unwrap(), 2);
    assert_eq!(*received_b.read().unwrap(), 1);
}

// --- History tests ---

#[test]
fn bus_history_records_events() {
    let bus = EventBus::new(100);
    bus.emit_simple("a", "1".into(), "s");
    bus.emit_simple("b", "2".into(), "s");

    let history = bus.history();
    assert_eq!(history.len(), 2);
}

#[test]
fn bus_history_by_type() {
    let bus = EventBus::new(100);
    bus.emit_simple("a", "1".into(), "s");
    bus.emit_simple("b", "2".into(), "s");
    bus.emit_simple("a", "3".into(), "s");

    assert_eq!(bus.history_by_type("a").len(), 2);
    assert_eq!(bus.history_by_type("b").len(), 1);
    assert_eq!(bus.history_by_type("c").len(), 0);
}

#[test]
fn bus_clear_history() {
    let bus = EventBus::new(100);
    bus.emit_simple("e", "d".into(), "s");
    assert!(!bus.history().is_empty());

    bus.clear_history();
    assert!(bus.history().is_empty());
}

#[test]
fn bus_history_limit() {
    let bus = EventBus::new(3);
    for i in 0..5 {
        bus.emit_simple("e", format!("{}", i), "s");
    }
    let h = bus.history();
    assert_eq!(h.len(), 3);
    // oldest should be evicted
    assert_eq!(h[0].payload, "2");
}

// --- Handler count / event types ---

#[test]
fn bus_handler_count() {
    let bus = EventBus::new(10);
    assert_eq!(bus.handler_count("x"), 0);

    bus.subscribe("x", |_| {});
    assert_eq!(bus.handler_count("x"), 1);

    bus.subscribe("x", |_| {});
    assert_eq!(bus.handler_count("x"), 2);

    assert_eq!(bus.handler_count("y"), 0);
}

#[test]
fn bus_event_types() {
    let bus = EventBus::new(10);
    bus.subscribe("alpha", |_| {});
    bus.subscribe("beta", |_| {});

    let types = bus.event_types();
    assert!(types.contains(&"alpha".to_string()));
    assert!(types.contains(&"beta".to_string()));
}

// --- Default ---

#[test]
fn bus_default() {
    let bus = EventBus::default();
    bus.emit_simple("e", "d".into(), "s");
    assert_eq!(bus.history().len(), 1);
}

// --- Thread safety ---

#[test]
fn bus_thread_safe_emit() {
    let bus = Arc::new(EventBus::new(1000));
    let count = Arc::new(RwLock::new(0));

    let c = count.clone();
    bus.subscribe("inc", move |_| {
        *c.write().unwrap() += 1;
    });

    let mut handles = vec![];
    for _ in 0..10 {
        let bus = bus.clone();
        handles.push(std::thread::spawn(move || {
            bus.emit_simple("inc", "x".into(), "t");
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(*count.read().unwrap(), 10);
    assert_eq!(bus.history().len(), 10);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn history_never_exceeds_max(max in 1usize..50, events in 0usize..100) {
            let bus = EventBus::new(max);
            for i in 0..events {
                bus.emit_simple("e", format!("{}", i), "s");
            }
            prop_assert!(bus.history().len() <= max);
        }

        #[test]
        fn handler_count_matches_subscriptions(n in 0usize..20) {
            let bus = EventBus::new(10);
            for _ in 0..n {
                bus.subscribe("t", |_| {});
            }
            prop_assert_eq!(bus.handler_count("t"), n);
        }
    }
}
