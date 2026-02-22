//! File system watcher for real-time shiplog sync.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// Watcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Path to watch
    pub path: PathBuf,
    /// Whether to watch recursively
    #[serde(default = "default_true")]
    pub recursive: bool,
    /// Poll interval in seconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
}

fn default_true() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    2
}

/// File watcher for monitoring file changes
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: &WatcherConfig) -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(config.poll_interval)),
        )?;

        watcher.watch(&config.path, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Try to receive an event (non-blocking)
    pub fn try_recv(&self) -> Option<Result<Event, notify::Error>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
        }
    }
}

/// Event type for file changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
}

/// Kind of file event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEventKind {
    Created,
    Modified,
    Removed,
    Any,
}

impl From<&EventKind> for FileEventKind {
    fn from(kind: &EventKind) -> Self {
        match kind {
            EventKind::Create(_) => FileEventKind::Created,
            EventKind::Modify(_) => FileEventKind::Modified,
            EventKind::Remove(_) => FileEventKind::Removed,
            _ => FileEventKind::Any,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn watcher_config_default() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            path: temp_dir.path().to_path_buf(),
            recursive: true,
            poll_interval: 2,
        };
        assert!(config.recursive);
        assert_eq!(config.poll_interval, 2);
    }

    #[test]
    fn file_event_kind_from_event() {
        use notify::EventKind;
        let kind = FileEventKind::from(&EventKind::Create(notify::event::CreateKind::File));
        assert!(matches!(kind, FileEventKind::Created));
    }

    #[test]
    fn create_watcher() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config = WatcherConfig {
            path: temp_dir.path().to_path_buf(),
            recursive: true,
            poll_interval: 1,
        };

        let _watcher = FileWatcher::new(&config)?;

        // Create a test file to trigger an event
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        // Give the watcher a moment to detect the change
        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(())
    }
}
