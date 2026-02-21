//! Signal handling utilities for shiplog.
//!
//! This crate provides signal handling utilities for the shiplog ecosystem,
//! allowing graceful handling of OS signals like SIGINT, SIGTERM, etc.

use anyhow::Result;
use tokio::sync::mpsc;

/// Signal types that can be handled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Interrupt signal (Ctrl+C)
    Interrupt,
    /// Termination signal
    Terminate,
    /// Hangup signal
    Hangup,
}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Signal::Interrupt => write!(f, "Interrupt"),
            Signal::Terminate => write!(f, "Terminate"),
            Signal::Hangup => write!(f, "Hangup"),
        }
    }
}

/// A signal handler that listens for OS signals
pub struct SignalHandler {
    interrupt_tx: Option<mpsc::Sender<()>>,
    #[allow(dead_code)]
    terminate_tx: Option<mpsc::Sender<()>>,
}

impl SignalHandler {
    /// Creates a new signal handler
    pub fn new() -> Self {
        Self {
            interrupt_tx: None,
            terminate_tx: None,
        }
    }

    /// Creates a new signal handler with channels
    pub fn with_channels() -> (Self, mpsc::Receiver<()>, mpsc::Receiver<()>) {
        let (interrupt_tx, interrupt_rx) = mpsc::channel(1);
        let (terminate_tx, terminate_rx) = mpsc::channel(1);

        let handler = Self {
            interrupt_tx: Some(interrupt_tx),
            terminate_tx: Some(terminate_tx),
        };

        (handler, interrupt_rx, terminate_rx)
    }

    /// Starts listening for SIGINT and SIGTERM signals (Unix only)
    #[cfg(unix)]
    pub async fn listen(&mut self) -> Result<Signal> {
        use tokio::signal;

        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;

        tokio::select! {
            _ = sigint.recv() => {
                if let Some(tx) = &self.interrupt_tx {
                    let _ = tx.send(()).await;
                }
                Ok(Signal::Interrupt)
            }
            _ = sigterm.recv() => {
                if let Some(tx) = &self.terminate_tx {
                    let _ = tx.send(()).await;
                }
                Ok(Signal::Terminate)
            }
        }
    }

    /// Starts listening for Ctrl+C and termination signals (Windows)
    #[cfg(windows)]
    pub async fn listen(&mut self) -> Result<Signal> {
        use tokio::signal;

        tokio::select! {
            _ = signal::ctrl_c() => {
                if let Some(tx) = &self.interrupt_tx {
                    let _ = tx.send(()).await;
                }
                Ok(Signal::Interrupt)
            }
        }
    }

    /// Creates a future that waits for any shutdown signal
    #[cfg(unix)]
    pub async fn wait_for_signal() -> Result<Signal> {
        use tokio::signal;

        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;

        tokio::select! {
            _ = sigint.recv() => Ok(Signal::Interrupt),
            _ = sigterm.recv() => Ok(Signal::Terminate),
        }
    }

    /// Creates a future that waits for any shutdown signal (Windows)
    #[cfg(windows)]
    pub async fn wait_for_signal() -> Result<Signal> {
        use tokio::signal;

        signal::ctrl_c().await?;
        Ok(Signal::Interrupt)
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a shutdown signal receiver pair
pub fn create_shutdown_channel() -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    mpsc::channel(1)
}

/// Sets up a signal handler that sends to a channel when triggered
pub async fn setup_signal_handler(tx: mpsc::Sender<()>) -> Result<()> {
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let _ = SignalHandler::wait_for_signal().await;
        let _ = tx_clone.send(()).await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_enum_values() {
        assert_eq!(Signal::Interrupt, Signal::Interrupt);
        assert_eq!(Signal::Terminate, Signal::Terminate);
        assert_eq!(Signal::Hangup, Signal::Hangup);
    }

    #[test]
    fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        assert!(handler.interrupt_tx.is_none());
        assert!(handler.terminate_tx.is_none());
    }

    #[test]
    fn test_signal_handler_with_channels() {
        let (handler, rx1, rx2) = SignalHandler::with_channels();
        assert!(handler.interrupt_tx.is_some());
        assert!(handler.terminate_tx.is_some());
        // Receivers should be different
        assert!(!std::ptr::eq(&rx1, &rx2));
    }

    #[test]
    fn test_create_shutdown_channel() {
        let (tx, mut rx) = create_shutdown_channel();
        assert!(tx.blocking_send(()).is_ok());
        // Check that we can receive the signal
        assert!(rx.blocking_recv().is_some());
    }

    #[test]
    fn test_signal_equality() {
        let s1 = Signal::Interrupt;
        let s2 = Signal::Interrupt;
        let s3 = Signal::Terminate;

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_signal_display() {
        assert_eq!(format!("{}", Signal::Interrupt), "Interrupt");
        assert_eq!(format!("{}", Signal::Terminate), "Terminate");
        assert_eq!(format!("{}", Signal::Hangup), "Hangup");
    }
}
