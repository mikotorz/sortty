use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub file_name: String,
    /// Lowercased extension without the leading dot, if any.
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub modified: DateTime<Utc>,
    pub created: Option<DateTime<Utc>>,
}
