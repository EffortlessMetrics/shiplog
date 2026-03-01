use shiplog_pubsub::{Message, PubSub};
use std::sync::{Arc, RwLock};

// --- Basic pub/sub ---

#[test]
fn publish_without_subscribers() {
    let ps: PubSub<String> = PubSub::new();
    let msg = ps.publish("topic", "hello".into());
    assert_eq!(msg.topic, "topic");
    assert_eq!(msg.payload, "hello");
    assert!(!msg.id.is_empty());
}

#[test]
fn subscribe_and_publish() {
    let ps: PubSub<String> = PubSub::new();
    let received = Arc::new(RwLock::new(Vec::new()));
    let r = received.clone();

    ps.subscribe("t", move |msg: Message<String>| {
        r.write().unwrap().push(msg.payload.clone());
    });

    ps.publish("t", "a".into());
    ps.publish("t", "b".into());

    let msgs = received.read().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0], "a");
    assert_eq!(msgs[1], "b");
}

#[test]
fn publish_to_different_topic_does_not_notify() {
    let ps: PubSub<String> = PubSub::new();
    let received = Arc::new(RwLock::new(0));
    let r = received.clone();

    ps.subscribe("topic-a", move |_: Message<String>| {
        *r.write().unwrap() += 1;
    });

    ps.publish("topic-b", "x".into());
    assert_eq!(*received.read().unwrap(), 0);
}

// --- Multiple subscribers ---

#[test]
fn multiple_subscribers_same_topic() {
    let ps: PubSub<i32> = PubSub::new();
    let count = Arc::new(RwLock::new(0));

    for _ in 0..4 {
        let c = count.clone();
        ps.subscribe("t", move |_: Message<i32>| {
            *c.write().unwrap() += 1;
        });
    }

    ps.publish("t", 42);
    assert_eq!(*count.read().unwrap(), 4);
}

#[test]
fn multiple_topics_independent() {
    let ps: PubSub<String> = PubSub::new();
    let a_count = Arc::new(RwLock::new(0));
    let b_count = Arc::new(RwLock::new(0));

    let ac = a_count.clone();
    ps.subscribe("a", move |_: Message<String>| {
        *ac.write().unwrap() += 1;
    });

    let bc = b_count.clone();
    ps.subscribe("b", move |_: Message<String>| {
        *bc.write().unwrap() += 1;
    });

    ps.publish("a", "x".into());
    ps.publish("b", "y".into());
    ps.publish("a", "z".into());

    assert_eq!(*a_count.read().unwrap(), 2);
    assert_eq!(*b_count.read().unwrap(), 1);
}

// --- Subscriber count ---

#[test]
fn subscriber_count_empty() {
    let ps: PubSub<String> = PubSub::new();
    assert_eq!(ps.subscriber_count("any"), 0);
}

#[test]
fn subscriber_count_increments() {
    let ps: PubSub<String> = PubSub::new();
    ps.subscribe("t", |_: Message<String>| {});
    ps.subscribe("t", |_: Message<String>| {});
    assert_eq!(ps.subscriber_count("t"), 2);
    assert_eq!(ps.subscriber_count("other"), 0);
}

// --- Topics ---

#[test]
fn topics_lists_subscribed() {
    let ps: PubSub<String> = PubSub::new();
    ps.subscribe("alpha", |_: Message<String>| {});
    ps.subscribe("beta", |_: Message<String>| {});

    let topics = ps.topics();
    assert!(topics.contains(&"alpha".to_string()));
    assert!(topics.contains(&"beta".to_string()));
}

#[test]
fn topics_empty_initially() {
    let ps: PubSub<String> = PubSub::new();
    assert!(ps.topics().is_empty());
}

// --- Message IDs ---

#[test]
fn message_ids_are_unique() {
    let ps: PubSub<String> = PubSub::new();
    let ids: Vec<String> = (0..10)
        .map(|i| ps.publish("t", format!("{}", i)).id)
        .collect();

    // Collect into a set; all should be unique
    let set: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(set.len(), ids.len());
}

// --- Default ---

#[test]
fn pubsub_default() {
    let ps: PubSub<String> = PubSub::default();
    let msg = ps.publish("t", "data".into());
    assert_eq!(msg.topic, "t");
}

// --- Generic type ---

#[test]
fn pubsub_with_numeric_payload() {
    let ps: PubSub<f64> = PubSub::new();
    let received = Arc::new(RwLock::new(Vec::new()));
    let r = received.clone();

    ps.subscribe("nums", move |msg: Message<f64>| {
        r.write().unwrap().push(msg.payload);
    });

    ps.publish("nums", 1.5);
    ps.publish("nums", 2.5);

    let vals = received.read().unwrap();
    assert!((vals[0] - 1.5).abs() < f64::EPSILON);
    assert!((vals[1] - 2.5).abs() < f64::EPSILON);
}

// --- Thread safety ---

#[test]
fn pubsub_thread_safe() {
    let ps = Arc::new(PubSub::<i32>::new());
    let count = Arc::new(RwLock::new(0));

    let c = count.clone();
    ps.subscribe("inc", move |_: Message<i32>| {
        *c.write().unwrap() += 1;
    });

    let mut handles = vec![];
    for i in 0..10 {
        let ps = ps.clone();
        handles.push(std::thread::spawn(move || {
            ps.publish("inc", i);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(*count.read().unwrap(), 10);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn subscriber_count_matches(n in 0usize..20) {
            let ps: PubSub<String> = PubSub::new();
            for _ in 0..n {
                ps.subscribe("t", |_: Message<String>| {});
            }
            prop_assert_eq!(ps.subscriber_count("t"), n);
        }

        #[test]
        fn publish_returns_correct_topic(topic in "[a-z]{1,20}") {
            let ps: PubSub<String> = PubSub::new();
            let msg = ps.publish(&topic, "data".into());
            prop_assert_eq!(&msg.topic, &topic);
        }
    }
}
