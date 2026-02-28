//! Integration tests for shiplog-signal: Signal enum, handler, shutdown channels.

use shiplog_signal::{Signal, SignalHandler, create_shutdown_channel};

// ── Signal enum ───────────────────────────────────────────────────────

#[test]
fn signal_display_interrupt() {
    assert_eq!(format!("{}", Signal::Interrupt), "Interrupt");
}

#[test]
fn signal_display_terminate() {
    assert_eq!(format!("{}", Signal::Terminate), "Terminate");
}

#[test]
fn signal_display_hangup() {
    assert_eq!(format!("{}", Signal::Hangup), "Hangup");
}

#[test]
fn signal_equality() {
    assert_eq!(Signal::Interrupt, Signal::Interrupt);
    assert_ne!(Signal::Interrupt, Signal::Terminate);
    assert_ne!(Signal::Terminate, Signal::Hangup);
}

#[test]
fn signal_clone() {
    let s = Signal::Terminate;
    let cloned = s;
    assert_eq!(s, cloned);
}

#[test]
fn signal_debug() {
    let dbg = format!("{:?}", Signal::Interrupt);
    assert!(dbg.contains("Interrupt"));
}

// ── SignalHandler: creation ───────────────────────────────────────────

#[test]
fn handler_new_creates_instance() {
    let _handler = SignalHandler::new();
}

#[test]
fn handler_default_creates_instance() {
    let _handler = SignalHandler::default();
}

#[test]
fn handler_with_channels_returns_receivers() {
    let (_handler, _rx1, _rx2) = SignalHandler::with_channels();
}

// ── Shutdown channel ──────────────────────────────────────────────────

#[test]
fn shutdown_channel_send_receive() {
    let (tx, mut rx) = create_shutdown_channel();
    tx.blocking_send(()).unwrap();
    assert!(rx.blocking_recv().is_some());
}

#[test]
fn shutdown_channel_closed_on_drop() {
    let (tx, mut rx) = create_shutdown_channel();
    drop(tx);
    assert!(rx.blocking_recv().is_none());
}

#[tokio::test]
async fn shutdown_channel_async_send_receive() {
    let (tx, mut rx) = create_shutdown_channel();
    tx.send(()).await.unwrap();
    assert!(rx.recv().await.is_some());
}

#[tokio::test]
async fn shutdown_channel_async_closed() {
    let (tx, mut rx) = create_shutdown_channel();
    drop(tx);
    assert!(rx.recv().await.is_none());
}

// ── SignalHandler: with_channels – receivers are live ──────────────────

#[tokio::test]
async fn handler_with_channels_receivers_close_on_handler_drop() {
    let (handler, mut interrupt_rx, mut terminate_rx) = SignalHandler::with_channels();
    drop(handler); // dropping handler drops senders
    assert!(interrupt_rx.recv().await.is_none());
    assert!(terminate_rx.recv().await.is_none());
}

// ── Multiple shutdown channels ────────────────────────────────────────

#[tokio::test]
async fn multiple_shutdown_channels_independent() {
    let (tx1, mut rx1) = create_shutdown_channel();
    let (tx2, mut rx2) = create_shutdown_channel();

    tx1.send(()).await.unwrap();
    assert!(rx1.recv().await.is_some());

    // tx2 was not sent to, so dropping should close
    drop(tx2);
    assert!(rx2.recv().await.is_none());
}

// ── Signal: all variants covered ──────────────────────────────────────

#[test]
fn all_signal_variants() {
    let signals = [Signal::Interrupt, Signal::Terminate, Signal::Hangup];
    assert_eq!(signals.len(), 3);

    for s in &signals {
        let display = format!("{s}");
        assert!(!display.is_empty());
    }
}
