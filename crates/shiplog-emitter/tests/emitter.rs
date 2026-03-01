use shiplog_emitter::{Emitter, SharedEmitter, shared};
use std::sync::{Arc, Mutex};

// --- Basic emitter tests ---

#[test]
fn emitter_new_has_no_listeners() {
    let emitter: Emitter<String> = Emitter::new();
    assert!(!emitter.has_listeners("any"));
}

#[test]
fn emitter_default_has_no_listeners() {
    let emitter: Emitter<String> = Emitter::default();
    assert!(!emitter.has_listeners("any"));
}

#[test]
fn emitter_on_and_emit() {
    let mut emitter: Emitter<String> = Emitter::new();
    let received = Arc::new(Mutex::new(Vec::new()));
    let r = received.clone();

    emitter.on("event", move |data| {
        r.lock().unwrap().push(data.clone());
    });

    emitter.emit("event", &"hello".to_string());
    emitter.emit("event", &"world".to_string());

    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0], "hello");
    assert_eq!(msgs[1], "world");
}

#[test]
fn emitter_emit_no_listeners_is_noop() {
    let mut emitter: Emitter<i32> = Emitter::new();
    // Should not panic
    emitter.emit("no-event", &42);
}

// --- Multiple listeners ---

#[test]
fn emitter_multiple_listeners_same_event() {
    let mut emitter: Emitter<i32> = Emitter::new();
    let count = Arc::new(Mutex::new(0));

    for _ in 0..3 {
        let c = count.clone();
        emitter.on("ev", move |_| {
            *c.lock().unwrap() += 1;
        });
    }

    emitter.emit("ev", &1);
    assert_eq!(*count.lock().unwrap(), 3);
}

#[test]
fn emitter_different_events_independent() {
    let mut emitter: Emitter<String> = Emitter::new();
    let a_count = Arc::new(Mutex::new(0));
    let b_count = Arc::new(Mutex::new(0));

    let ac = a_count.clone();
    emitter.on("a", move |_| {
        *ac.lock().unwrap() += 1;
    });

    let bc = b_count.clone();
    emitter.on("b", move |_| {
        *bc.lock().unwrap() += 1;
    });

    emitter.emit("a", &"x".to_string());
    emitter.emit("a", &"y".to_string());
    emitter.emit("b", &"z".to_string());

    assert_eq!(*a_count.lock().unwrap(), 2);
    assert_eq!(*b_count.lock().unwrap(), 1);
}

// --- off / has_listeners ---

#[test]
fn emitter_off_removes_listeners() {
    let mut emitter: Emitter<String> = Emitter::new();
    let received = Arc::new(Mutex::new(Vec::new()));
    let r = received.clone();

    emitter.on("ev", move |data| {
        r.lock().unwrap().push(data.clone());
    });

    emitter.emit("ev", &"before".to_string());
    emitter.off("ev");
    emitter.emit("ev", &"after".to_string());

    let msgs = received.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], "before");
}

#[test]
fn emitter_off_nonexistent_event_is_noop() {
    let mut emitter: Emitter<String> = Emitter::new();
    emitter.off("nonexistent"); // Should not panic
}

#[test]
fn emitter_has_listeners_reflects_state() {
    let mut emitter: Emitter<String> = Emitter::new();

    assert!(!emitter.has_listeners("ev"));

    emitter.on("ev", |_| {});
    assert!(emitter.has_listeners("ev"));

    emitter.off("ev");
    assert!(!emitter.has_listeners("ev"));
}

// --- SharedEmitter tests ---

#[test]
fn shared_emitter_creation() {
    let _emitter: SharedEmitter<String> = shared();
}

#[test]
fn shared_emitter_on_and_emit() {
    let emitter: SharedEmitter<i32> = shared();
    let received = Arc::new(Mutex::new(Vec::new()));
    let r = received.clone();

    emitter.write().unwrap().on("num", move |v| {
        r.lock().unwrap().push(*v);
    });

    emitter.write().unwrap().emit("num", &42);
    emitter.write().unwrap().emit("num", &99);

    let vals = received.lock().unwrap();
    assert_eq!(*vals, vec![42, 99]);
}

#[test]
fn shared_emitter_thread_safe() {
    let emitter: SharedEmitter<i32> = shared();
    let sum = Arc::new(Mutex::new(0));

    let s = sum.clone();
    emitter.write().unwrap().on("add", move |v| {
        *s.lock().unwrap() += v;
    });

    let emitter = Arc::new(emitter);
    let mut handles = vec![];

    for i in 0..5 {
        let e = emitter.clone();
        handles.push(std::thread::spawn(move || {
            e.write().unwrap().emit("add", &i);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Sum of 0+1+2+3+4 = 10
    assert_eq!(*sum.lock().unwrap(), 10);
}

// --- FnMut listeners ---

#[test]
fn emitter_fnmut_listener_accumulates() {
    let mut emitter: Emitter<i32> = Emitter::new();
    let sum = Arc::new(Mutex::new(0i32));
    let s = sum.clone();

    emitter.on("val", move |v| {
        *s.lock().unwrap() += v;
    });

    for i in 1..=5 {
        emitter.emit("val", &i);
    }

    assert_eq!(*sum.lock().unwrap(), 15);
}

// --- Generic types ---

#[test]
fn emitter_with_struct_payload() {
    #[derive(Clone, Debug, PartialEq)]
    struct Payload {
        x: i32,
        y: String,
    }

    let mut emitter: Emitter<Payload> = Emitter::new();
    let received = Arc::new(Mutex::new(Vec::new()));
    let r = received.clone();

    emitter.on("payload", move |p| {
        r.lock().unwrap().push(p.clone());
    });

    let p = Payload {
        x: 1,
        y: "hello".to_string(),
    };
    emitter.emit("payload", &p);

    let msgs = received.lock().unwrap();
    assert_eq!(msgs[0], p);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn emit_count_matches_listeners(n in 0usize..10, events in 1usize..5) {
            let mut emitter: Emitter<i32> = Emitter::new();
            let count = Arc::new(Mutex::new(0usize));

            for _ in 0..n {
                let c = count.clone();
                emitter.on("ev", move |_| {
                    *c.lock().unwrap() += 1;
                });
            }

            for _ in 0..events {
                emitter.emit("ev", &0);
            }

            prop_assert_eq!(*count.lock().unwrap(), n * events);
        }

        #[test]
        fn has_listeners_after_on(event in "[a-z]{1,10}") {
            let mut emitter: Emitter<String> = Emitter::new();
            emitter.on(&event, |_| {});
            prop_assert!(emitter.has_listeners(&event));
        }
    }
}
