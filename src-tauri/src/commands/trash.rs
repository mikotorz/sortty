use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::commands::blocking;
use crate::config::settings::{self, AppSettings};
use crate::engine::scanner::{self, ScanOptions};
use crate::error::AppError;

/// Which tool-owned staging folder to target. Both are move-not-delete
/// staging areas (see ADR 0002) and mechanically identical to empty, but are
/// kept as separately-labeled actions in the UI since they mean different
/// things to a user (confirmed duplicates vs. stale-but-maybe-still-wanted
/// files).
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingKind {
    Trash,
    Archive,
}

#[derive(Debug, Serialize)]
pub struct EmptyResult {
    pub deleted_files: usize,
    pub freed_bytes: u64,
}

fn folder_name_for(kind: StagingKind, settings: &AppSettings) -> &str {
    match kind {
        StagingKind::Trash => &settings.trash.staging_folder_name,
        StagingKind::Archive => &settings.trash.archive_folder_name,
    }
}

/// Counts what emptying `kind`'s staging folder under `root` would delete,
/// without deleting anything — used to show the user a dry-run preview
/// before the irreversible confirm.
pub fn preview_staging_folder_at(
    root: &Path,
    kind: StagingKind,
    settings: &AppSettings,
) -> Result<EmptyResult, AppError> {
    let target = root.join(folder_name_for(kind, settings));
    if !target.exists() {
        return Ok(EmptyResult {
            deleted_files: 0,
            freed_bytes: 0,
        });
    }
    let entries = scanner::scan(
        &target,
        &ScanOptions {
            include_subfolders: true,
            exclude: Vec::new(),
            exclude_folders: Vec::new(),
        },
    )?;
    Ok(EmptyResult {
        deleted_files: entries.len(),
        freed_bytes: entries.iter().map(|e| e.size_bytes).sum(),
    })
}

/// Permanently deletes `kind`'s staging folder (and everything in it) under
/// `root`. This is the app's first real, non-backed-up filesystem delete —
/// every other `remove_file` call in the codebase only ever fires after an
/// equivalent copy has already landed elsewhere.
pub fn empty_staging_folder_at(
    root: &Path,
    kind: StagingKind,
    settings: &AppSettings,
) -> Result<EmptyResult, AppError> {
    let result = preview_staging_folder_at(root, kind, settings)?;
    let target = root.join(folder_name_for(kind, settings));
    if target.exists() {
        if !target.starts_with(root) {
            return Err(AppError::Other(format!(
                "refusing to delete {} — it isn't inside {}",
                target.display(),
                root.display()
            )));
        }
        std::fs::remove_dir_all(&target).map_err(|e| AppError::io(target.clone(), e))?;
    }
    Ok(result)
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
pub async fn preview_staging_folder(
    app: AppHandle,
    root: String,
    kind: StagingKind,
) -> Result<EmptyResult, AppError> {
    let settings = settings::load_settings(&config_dir(&app)?)?;
    blocking(move || preview_staging_folder_at(Path::new(&root), kind, &settings)).await
}

#[tauri::command]
pub async fn empty_staging_folder(
    app: AppHandle,
    root: String,
    kind: StagingKind,
) -> Result<EmptyResult, AppError> {
    let settings = settings::load_settings(&config_dir(&app)?)?;
    blocking(move || empty_staging_folder_at(Path::new(&root), kind, &settings)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn preview_returns_zero_when_the_folder_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let result =
            preview_staging_folder_at(dir.path(), StagingKind::Trash, &AppSettings::default())
                .unwrap();
        assert_eq!(result.deleted_files, 0);
        assert_eq!(result.freed_bytes, 0);
    }

    #[test]
    fn preview_counts_files_and_bytes_in_the_staging_folder() {
        let dir = tempfile::tempdir().unwrap();
        let trash = dir.path().join(".sortty-trash");
        fs::create_dir_all(trash.join("nested")).unwrap();
        fs::write(trash.join("a.txt"), b"12345").unwrap();
        fs::write(trash.join("nested/b.txt"), b"1234567890").unwrap();

        let result =
            preview_staging_folder_at(dir.path(), StagingKind::Trash, &AppSettings::default())
                .unwrap();

        assert_eq!(result.deleted_files, 2);
        assert_eq!(result.freed_bytes, 15);
    }

    #[test]
    fn empty_deletes_the_staging_folder_and_its_contents() {
        let dir = tempfile::tempdir().unwrap();
        let trash = dir.path().join(".sortty-trash");
        fs::create_dir_all(&trash).unwrap();
        fs::write(trash.join("a.txt"), b"a").unwrap();

        let result =
            empty_staging_folder_at(dir.path(), StagingKind::Trash, &AppSettings::default())
                .unwrap();

        assert_eq!(result.deleted_files, 1);
        assert!(!trash.exists());
    }

    #[test]
    fn empty_is_a_no_op_when_the_folder_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let result =
            empty_staging_folder_at(dir.path(), StagingKind::Trash, &AppSettings::default())
                .unwrap();
        assert_eq!(result.deleted_files, 0);
    }

    #[test]
    fn empty_only_touches_the_named_staging_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("keep.txt"), b"keep").unwrap();
        let trash = dir.path().join(".sortty-trash");
        fs::create_dir_all(&trash).unwrap();
        fs::write(trash.join("gone.txt"), b"gone").unwrap();
        let archive = dir.path().join(".sortty-archive");
        fs::create_dir_all(&archive).unwrap();
        fs::write(archive.join("stays.txt"), b"stays").unwrap();

        empty_staging_folder_at(dir.path(), StagingKind::Trash, &AppSettings::default()).unwrap();

        assert!(dir.path().join("keep.txt").exists());
        assert!(!trash.exists());
        assert!(archive.join("stays.txt").exists());
    }

    #[test]
    fn trash_and_archive_use_their_own_configured_folder_names() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = AppSettings::default();
        settings.trash.staging_folder_name = "MyTrash".to_string();
        settings.trash.archive_folder_name = "MyArchive".to_string();
        fs::create_dir_all(dir.path().join("MyTrash")).unwrap();
        fs::write(dir.path().join("MyTrash/a.txt"), b"a").unwrap();

        let trash_result =
            preview_staging_folder_at(dir.path(), StagingKind::Trash, &settings).unwrap();
        let archive_result =
            preview_staging_folder_at(dir.path(), StagingKind::Archive, &settings).unwrap();

        assert_eq!(trash_result.deleted_files, 1);
        assert_eq!(archive_result.deleted_files, 0);
    }
}
