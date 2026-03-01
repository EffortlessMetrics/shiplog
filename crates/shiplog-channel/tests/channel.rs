//! Integration tests for shiplog-channel: concurrency, edge cases, multi-producer/consumer.

use shiplog_channel::{
    BroadcastReceiver, BroadcastSender, ChannelBuilder, ChannelMessage, ChannelMetadata,
    ChannelType, MpscReceiver, MpscSender, create_broadcast_channel, create_mpsc_channel,
};
use std::sync::Arc;
use tokio::sync::Barrier;

// ── mpsc: multi-producer single-consumer ──────────────────────────────

#[tokio::test]
async fn mpsc_multiple_producers() {
    let (tx, rx) = create_mpsc_channel::<u64>(64);
    let mut receiver = MpscReceiver::new(rx);

    let mut handles = Vec::new();
    for id in 0..10u64 {
        let sender = MpscSender::new(tx.clone());
        handles.push(tokio::spawn(async move {
            for i in 0..100u64 {
                sender.send(id * 1000 + i).await.unwrap();
            }
        }));
    }
    drop(tx); // close original sender so receiver finishes

    let mut values = Vec::new();
    while let Some(v) = receiver.recv().await {
        values.push(v);
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(values.len(), 1000, "should receive exactly 1000 messages");
    // Each producer sent 100 values; total unique values = 1000
    values.sort();
    values.dedup();
    assert_eq!(values.len(), 1000);
}

#[tokio::test]
async fn mpsc_single_message() {
    let (tx, rx) = create_mpsc_channel::<&str>(1);
    let sender = MpscSender::new(tx);
    let mut receiver = MpscReceiver::new(rx);

    sender.send("hello").await.unwrap();
    assert_eq!(receiver.recv().await, Some("hello"));
}

#[tokio::test]
async fn mpsc_sender_dropped_signals_none() {
    let (tx, rx) = create_mpsc_channel::<i32>(4);
    let mut receiver = MpscReceiver::new(rx);
    drop(tx);
    assert_eq!(receiver.recv().await, None);
}

#[tokio::test]
async fn mpsc_capacity_reported() {
    let (tx, _rx) = create_mpsc_channel::<i32>(8);
    let sender = MpscSender::new(tx);
    assert_eq!(sender.capacity(), 8);
}

#[tokio::test]
async fn mpsc_is_closed_after_receiver_drop() {
    let (tx, rx) = create_mpsc_channel::<i32>(4);
    let sender = MpscSender::new(tx);
    assert!(!sender.is_closed());
    drop(rx);
    assert!(sender.is_closed());
}

// ── broadcast: multi-subscriber ───────────────────────────────────────

#[tokio::test]
async fn broadcast_multiple_receivers() {
    let (tx, rx1) = create_broadcast_channel::<i32>(16);
    let rx2 = tx.subscribe();

    let sender = BroadcastSender::new(tx);
    let mut r1 = BroadcastReceiver::new(rx1);
    let mut r2 = BroadcastReceiver::new(rx2);

    sender.send(42).unwrap();

    assert_eq!(r1.recv().await.unwrap(), 42);
    assert_eq!(r2.recv().await.unwrap(), 42);
}

#[tokio::test]
async fn broadcast_receiver_count() {
    let (tx, _rx1) = create_broadcast_channel::<i32>(16);
    let _rx2 = tx.subscribe();
    let sender = BroadcastSender::new(tx);
    assert_eq!(sender.receiver_count(), 2);
}

#[tokio::test]
async fn broadcast_no_receivers_returns_err() {
    let (tx, rx) = create_broadcast_channel::<i32>(4);
    drop(rx);
    let sender = BroadcastSender::new(tx);
    assert!(sender.send(1).is_err());
}

#[tokio::test]
async fn broadcast_concurrent_subscribers() {
    let (tx, _rx) = create_broadcast_channel::<u64>(128);
    let sender = Arc::new(BroadcastSender::new(tx.clone()));
    let barrier = Arc::new(Barrier::new(6)); // 5 receivers + 1 sender

    let mut handles = Vec::new();
    for _ in 0..5 {
        let rx = tx.subscribe();
        let mut receiver = BroadcastReceiver::new(rx);
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            let mut received = Vec::new();
            for _ in 0..50 {
                if let Ok(v) = receiver.recv().await {
                    received.push(v);
                }
            }
            received
        }));
    }

    // Sender task
    let b = barrier.clone();
    let s = sender.clone();
    let sender_handle = tokio::spawn(async move {
        b.wait().await;
        for i in 0..50u64 {
            let _ = s.send(i);
            tokio::task::yield_now().await;
        }
    });

    sender_handle.await.unwrap();
    for h in handles {
        let received = h.await.unwrap();
        assert!(!received.is_empty(), "each subscriber should get messages");
    }
}

// ── ChannelMessage ────────────────────────────────────────────────────

#[test]
fn channel_message_default_metadata() {
    let msg = ChannelMessage::new("data");
    assert_eq!(msg.payload, "data");
    assert_eq!(msg.metadata.sender_id, None);
    assert_eq!(msg.metadata.priority, 0);
    assert_eq!(msg.metadata.timestamp_ms, 0);
}

#[test]
fn channel_message_builder_chain() {
    let msg = ChannelMessage::new(99u8)
        .with_sender("worker-3")
        .with_priority(10);
    assert_eq!(msg.metadata.sender_id.as_deref(), Some("worker-3"));
    assert_eq!(msg.metadata.priority, 10);
}

#[test]
fn channel_metadata_default() {
    let meta = ChannelMetadata::default();
    assert_eq!(meta.sender_id, None);
    assert_eq!(meta.timestamp_ms, 0);
    assert_eq!(meta.priority, 0);
}

// ── ChannelBuilder / ChannelConfig ────────────────────────────────────

#[test]
fn builder_default_values() {
    let cfg = ChannelBuilder::default().build();
    assert_eq!(cfg.buffer_size, 100);
    assert_eq!(cfg.channel_type, ChannelType::Mpsc);
    assert_eq!(cfg.name, "default");
}

#[test]
fn builder_custom_values() {
    let cfg = ChannelBuilder::new()
        .buffer_size(1)
        .channel_type(ChannelType::Broadcast)
        .name("tiny")
        .build();
    assert_eq!(cfg.buffer_size, 1);
    assert_eq!(cfg.channel_type, ChannelType::Broadcast);
    assert_eq!(cfg.name, "tiny");
}

#[test]
fn channel_type_equality() {
    assert_eq!(ChannelType::Mpsc, ChannelType::Mpsc);
    assert_ne!(ChannelType::Mpsc, ChannelType::Broadcast);
}

// ── mpsc: ordered delivery ────────────────────────────────────────────

#[tokio::test]
async fn mpsc_preserves_order_from_single_producer() {
    let (tx, rx) = create_mpsc_channel::<usize>(256);
    let sender = MpscSender::new(tx);
    let mut receiver = MpscReceiver::new(rx);

    let n = 200;
    for i in 0..n {
        sender.send(i).await.unwrap();
    }
    drop(sender);

    let mut prev = None;
    while let Some(v) = receiver.recv().await {
        if let Some(p) = prev {
            assert!(v > p, "values should arrive in order");
        }
        prev = Some(v);
    }
    assert_eq!(prev, Some(n - 1));
}

// ── mpsc: buffer-size-1 edge case ─────────────────────────────────────

#[tokio::test]
async fn mpsc_buffer_size_one() {
    let (tx, rx) = create_mpsc_channel::<i32>(1);
    let sender = MpscSender::new(tx);
    let mut receiver = MpscReceiver::new(rx);

    let send = tokio::spawn(async move {
        for i in 0..10 {
            sender.send(i).await.unwrap();
        }
    });

    let mut received = Vec::new();
    for _ in 0..10 {
        if let Some(v) = receiver.recv().await {
            received.push(v);
        }
    }

    send.await.unwrap();
    assert_eq!(received.len(), 10);
}

// ── ChannelMessage through channel ────────────────────────────────────

#[tokio::test]
async fn channel_message_sent_and_received() {
    let (tx, rx) = create_mpsc_channel::<ChannelMessage<String>>(4);
    let sender = MpscSender::new(tx);
    let mut receiver = MpscReceiver::new(rx);

    let msg = ChannelMessage::new("payload".to_string())
        .with_sender("test")
        .with_priority(3);
    sender.send(msg).await.unwrap();

    let received = receiver.recv().await.unwrap();
    assert_eq!(received.payload, "payload");
    assert_eq!(received.metadata.sender_id.as_deref(), Some("test"));
    assert_eq!(received.metadata.priority, 3);
}
