use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;

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

pub fn scan_folder_at(root_path: &Path, options: ScanOptions) -> Result<ScanResult, AppError> {
    let entries = scanner::scan(root_path, &options)?;
    let total_bytes = entries.iter().map(|e| e.size_bytes).sum();
    Ok(ScanResult {
        total_files: entries.len(),
        total_bytes,
        entries,
        scanned_at: Utc::now(),
    })
}

#[tauri::command]
pub async fn scan_folder(
    root: String,
    options: Option<ScanOptions>,
) -> Result<ScanResult, AppError> {
    scan_folder_at(Path::new(&root), options.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reports_correct_totals_for_known_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"12345").unwrap();
        fs::write(dir.path().join("b.txt"), b"1234567890").unwrap();

        let result = scan_folder_at(dir.path(), ScanOptions::default()).unwrap();

        assert_eq!(result.total_files, 2);
        assert_eq!(result.total_bytes, 15);
    }

    #[test]
    fn errors_when_root_does_not_exist() {
        let missing = Path::new("D:\\definitely-not-a-real-path-sortty-test");
        assert!(scan_folder_at(missing, ScanOptions::default()).is_err());
    }

    #[test]
    fn default_options_exclude_trash_and_archive_folders() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::create_dir(dir.path().join(".sortty-trash")).unwrap();
        fs::write(dir.path().join(".sortty-trash/b.txt"), b"b").unwrap();

        let result = scan_folder_at(dir.path(), ScanOptions::default()).unwrap();

        assert_eq!(result.total_files, 1);
    }
}
