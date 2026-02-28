use proptest::prelude::*;
use shiplog_actor::*;

// ── ActorConfig & Builder ───────────────────────────────────────────

#[test]
fn builder_default_matches_config_default() {
    let from_builder = ActorBuilder::default().build();
    let from_config = ActorConfig::default();
    assert_eq!(from_builder.name, from_config.name);
    assert_eq!(from_builder.buffer_size, from_config.buffer_size);
    assert_eq!(from_builder.mail_box_type, from_config.mail_box_type);
}

proptest! {
    #[test]
    fn builder_preserves_name(name in "[a-z]{1,20}") {
        let config = ActorBuilder::new().name(&name).build();
        prop_assert_eq!(config.name, name);
    }

    #[test]
    fn builder_preserves_buffer_size(size in 1usize..10_000) {
        let config = ActorBuilder::new().buffer_size(size).build();
        prop_assert_eq!(config.buffer_size, size);
    }
}

// ── ActorMessage ────────────────────────────────────────────────────

#[test]
fn message_sender_id_chaining() {
    let msg = ActorMessage::new(99)
        .with_sender_id("alice")
        .with_sender_id("bob");
    assert_eq!(msg.sender_id.as_deref(), Some("bob"));
}

proptest! {
    #[test]
    fn message_payload_roundtrip(val: i64) {
        let msg = ActorMessage::new(val);
        prop_assert_eq!(msg.payload, val);
        prop_assert!(msg.sender_id.is_none());
    }

    #[test]
    fn message_with_sender_roundtrip(val: i64, sender in "[a-z]{1,10}") {
        let msg = ActorMessage::new(val).with_sender_id(&sender);
        prop_assert_eq!(msg.payload, val);
        prop_assert_eq!(msg.sender_id.as_deref(), Some(sender.as_str()));
    }
}

// ── ActorState ──────────────────────────────────────────────────────

#[test]
fn state_increment_starts_at_zero() {
    let mut s = ActorState::new("hello");
    assert_eq!(s.message_count, 0);
    s.increment();
    assert_eq!(s.message_count, 1);
}

proptest! {
    #[test]
    fn state_increment_n_times(n in 0u64..500) {
        let mut s = ActorState::new(());
        for _ in 0..n {
            s.increment();
        }
        prop_assert_eq!(s.message_count, n);
    }
}

// ── Bounded channel ─────────────────────────────────────────────────

#[tokio::test]
async fn bounded_channel_fifo_order() {
    let (handle, mut actor_ref) = create_bounded_actor_channel::<u32>(16);
    for i in 0..10 {
        handle.send(i).await.unwrap();
    }
    for i in 0..10 {
        assert_eq!(actor_ref.recv().await, Some(i));
    }
}

#[tokio::test]
async fn handle_is_closed_after_receiver_drop() {
    let (handle, actor_ref) = create_bounded_actor_channel::<i32>(4);
    assert!(!handle.is_closed());
    drop(actor_ref);
    assert!(handle.is_closed());
}

#[tokio::test]
async fn recv_returns_none_after_sender_drop() {
    let (handle, mut actor_ref) = create_bounded_actor_channel::<i32>(4);
    handle.send(1).await.unwrap();
    drop(handle);
    assert_eq!(actor_ref.recv().await, Some(1));
    assert_eq!(actor_ref.recv().await, None);
}

#[tokio::test]
async fn debug_impl_does_not_panic() {
    let (handle, _rx) = create_bounded_actor_channel::<i32>(4);
    let dbg = format!("{:?}", handle);
    assert!(dbg.contains("ActorHandle"));
}

// ── Unbounded channel ───────────────────────────────────────────────

#[tokio::test]
async fn unbounded_channel_basic() {
    let (tx, mut rx) = create_unbounded_actor_channel::<String>();
    tx.send("hello".into()).unwrap();
    tx.send("world".into()).unwrap();
    assert_eq!(rx.recv().await, Some("hello".into()));
    assert_eq!(rx.recv().await, Some("world".into()));
}

// ── Concurrency: sequential sends ───────────────────────────────────

#[tokio::test]
async fn many_sequential_sends_received_in_order() {
    let (handle, mut actor_ref) = create_bounded_actor_channel::<u64>(100);
    let n = 50u64;
    for i in 0..n {
        handle.send(i).await.unwrap();
    }
    drop(handle);
    let mut received = Vec::new();
    while let Some(v) = actor_ref.recv().await {
        received.push(v);
    }
    assert_eq!(received, (0..n).collect::<Vec<_>>());
}

// ── MailBoxType equality ────────────────────────────────────────────

#[test]
fn mailbox_type_eq() {
    assert_eq!(MailBoxType::Bounded, MailBoxType::Bounded);
    assert_eq!(MailBoxType::Unbounded, MailBoxType::Unbounded);
    assert_ne!(MailBoxType::Bounded, MailBoxType::Unbounded);
}
