# shiplog-backup

Backup and restore functionality for shiplog data.

## Overview

Provides utilities for creating backups of shiplog data, managing backup metadata, and restoring from backups. Backups are stored as ZIP archives with metadata stored in separate JSON files.

## Features

- Create compressed ZIP backups of shiplog data directories
- Restore backups to any target location
- Backup metadata tracking with checksums
- List and manage existing backups
- Automatic backup deletion

## Usage

```rust
use shiplog_backup::BackupManager;
use std::path::Path;

// Create a backup manager
let manager = BackupManager::new("/backups");

// Create a backup
let backup = manager.create_backup(
    &Path::new("/data/shiplog"),
    "weekly-backup",
)?;

// List all backups
let backups = manager.list_backups()?;
for b in &backups {
    println!("Backup: {} - {} bytes", b.id, b.size_bytes);
}

// Restore a backup
manager.restore_backup(&backup_id, &Path::new("/restored"))?;

// Delete a backup
manager.delete_backup(&backup_id)?;
```

## License

MIT OR Apache-2.0
