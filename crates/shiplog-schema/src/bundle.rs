use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::RunId;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChecksum {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleManifest {
    pub run_id: RunId,
    pub generated_at: DateTime<Utc>,
    pub files: Vec<FileChecksum>,
}
