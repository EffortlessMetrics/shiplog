//! File system watching for changes in shiplog.

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// File change event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileEventType {
    Created,
    Modified,
    Removed,
    Renamed,
}

/// A file system change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub event_type: FileEventType,
}

/// Watch configuration
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Paths to watch
    pub paths: Vec<PathBuf>,
    /// Poll interval in seconds
    pub poll_interval: u64,
    /// Recursive watching
    pub recursive: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            poll_interval: 2,
            recursive: true,
        }
    }
}

/// File system watcher
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

impl FileWatcher {
    /// Create a new file watcher with the given configuration
    pub fn new(config: &WatchConfig) -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let notify_config =
            Config::default().with_poll_interval(Duration::from_secs(config.poll_interval));

        let mut watcher = RecommendedWatcher::new(tx, notify_config)
            .map_err(|e| anyhow::anyhow!("Failed to create watcher: {}", e))?;

        for path in &config.paths {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| anyhow::anyhow!("Failed to watch path {:?}: {}", path, e))?;
        }

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Try to receive a file change event (non-blocking).
    ///
    /// Skips noise events (e.g. metadata/access on Linux) that `convert_event`
    /// maps to `None`.
    pub fn try_recv(&self) -> Option<FileChange> {
        loop {
            match self.receiver.try_recv() {
                Ok(result) => {
                    if let Some(change) = result.ok().and_then(convert_event) {
                        return Some(change);
                    }
                    // Noise event — try again without blocking
                }
                Err(_) => return None,
            }
        }
    }

    /// Receive a file change event (blocking).
    ///
    /// Skips noise events (e.g. metadata/access on Linux) that `convert_event`
    /// maps to `None`.
    pub fn recv(&self) -> Option<FileChange> {
        loop {
            match self.receiver.recv() {
                Ok(result) => {
                    if let Some(change) = result.ok().and_then(convert_event) {
                        return Some(change);
                    }
                    // Noise event — keep waiting
                }
                Err(_) => return None,
            }
        }
    }

    /// Receive a file change event with a timeout.
    ///
    /// Skips noise events (e.g. metadata/access on Linux) that `convert_event`
    /// maps to `None`, retrying until a real event arrives or the timeout
    /// expires.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<FileChange> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.receiver.recv_timeout(remaining) {
                Ok(result) => {
                    if let Some(change) = result.ok().and_then(convert_event) {
                        return Some(change);
                    }
                    // Noise event — retry with remaining time
                }
                Err(_) => return None,
            }
        }
    }
}

fn convert_event(event: Event) -> Option<FileChange> {
    let event_type = match event.kind {
        notify::EventKind::Create(_) => FileEventType::Created,
        notify::EventKind::Modify(_) => FileEventType::Modified,
        notify::EventKind::Remove(_) => FileEventType::Removed,
        notify::EventKind::Other => return None,
        _ => return None,
    };

    // Use the first path from the event
    let path = event.paths.first()?.clone();

    Some(FileChange { path, event_type })
}

/// Watch a directory for changes and collect events
pub fn watch_directory<P: Into<PathBuf>>(path: P) -> anyhow::Result<FileWatcher> {
    let config = WatchConfig {
        paths: vec![path.into()],
        ..Default::default()
    };
    FileWatcher::new(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn watch_config_default() {
        let config = WatchConfig::default();
        assert!(config.paths.is_empty());
        assert_eq!(config.poll_interval, 2);
        assert!(config.recursive);
    }

    #[test]
    fn watch_created_file() {
        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();

        let config = WatchConfig {
            paths: vec![watch_path.clone()],
            poll_interval: 1,
            recursive: false,
        };

        let watcher = FileWatcher::new(&config).unwrap();

        // Create a file after a short delay
        thread::sleep(Duration::from_millis(100));
        let file_path = watch_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Try to receive the event
        let event = watcher.recv_timeout(Duration::from_secs(2));
        assert!(event.is_some());

        let event = event.unwrap();
        assert!(event.path.ends_with("test.txt"));
        assert_eq!(event.event_type, FileEventType::Created);
    }

    #[test]
    fn watch_modified_file() {
        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();

        // Create initial file
        let file_path = watch_path.join("test.txt");
        fs::write(&file_path, "initial").unwrap();

        let config = WatchConfig {
            paths: vec![watch_path.clone()],
            poll_interval: 1,
            recursive: false,
        };

        let watcher = FileWatcher::new(&config).unwrap();

        // Modify the file after a short delay
        thread::sleep(Duration::from_millis(100));
        fs::write(&file_path, "modified").unwrap();

        // Try to receive the event
        let event = watcher.recv_timeout(Duration::from_secs(2));
        assert!(event.is_some());

        let event = event.unwrap();
        assert!(event.path.ends_with("test.txt"));
        assert_eq!(event.event_type, FileEventType::Modified);
    }

    #[test]
    fn watch_removed_file() {
        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();

        // Create initial file
        let file_path = watch_path.join("test.txt");
        fs::write(&file_path, "test").unwrap();

        let config = WatchConfig {
            paths: vec![watch_path.clone()],
            poll_interval: 1,
            recursive: false,
        };

        let watcher = FileWatcher::new(&config).unwrap();

        // Remove the file after a short delay
        thread::sleep(Duration::from_millis(100));
        fs::remove_file(&file_path).unwrap();

        // Try to receive the event
        let event = watcher.recv_timeout(Duration::from_secs(2));
        assert!(event.is_some());

        let event = event.unwrap();
        assert!(event.path.ends_with("test.txt"));
        assert_eq!(event.event_type, FileEventType::Removed);
    }
}
