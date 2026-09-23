use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

use crate::domain::entry::FileEntry;
use crate::engine::scanner::{self, ScanOptions};
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub entries: Vec<FileEntry>,
    pub total_files: usize,
    pub total_bytes: u64,
    pub scanned_at: DateTime<Utc>,
}

#[tauri::command]
pub async fn scan_folder(
    root: String,
    options: Option<ScanOptions>,
) -> Result<ScanResult, AppError> {
    let root_path = PathBuf::from(root);
    let options = options.unwrap_or_default();
    let entries = scanner::scan(&root_path, &options)?;
    let total_bytes = entries.iter().map(|e| e.size_bytes).sum();
    Ok(ScanResult {
        total_files: entries.len(),
        total_bytes,
        entries,
        scanned_at: Utc::now(),
    })
}
