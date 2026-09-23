use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::apply::{executor, store};
use crate::commands::blocking;
use crate::config::settings::{self, TrashSettings};
use crate::domain::entry::FileEntry;
use crate::domain::plan::{staged_destination, Operation, OperationKind, Plan, PlanMode};
use crate::domain::run::RunRecord;
use crate::engine::scanner::{self, ScanOptions};
use crate::error::AppError;

/// Lists the current contents of an already-sorted destination folder, so the
/// user can pick files to delete. Reuses the ordinary scanner rather than a
/// separate listing routine — a destination folder is scanned the same way
/// any other folder is, just always recursively and always skipping the
/// tool's own staging folders.
pub fn browse_folder_at(folder: &Path, trash: &TrashSettings) -> Result<Vec<FileEntry>, AppError> {
    let options = ScanOptions {
        include_subfolders: true,
        exclude: Vec::new(),
        exclude_folders: Vec::new(),
    }
    .excluding(trash.staging_folder_names());
    scanner::scan(folder, &options)
}

fn load_trash_settings(app: &AppHandle) -> Result<TrashSettings, AppError> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(settings::load_settings(&config_dir)?.trash)
}

#[tauri::command]
pub async fn browse_folder(app: AppHandle, path: String) -> Result<Vec<FileEntry>, AppError> {
    let trash = load_trash_settings(&app)?;
    blocking(move || browse_folder_at(Path::new(&path), &trash)).await
}

/// "Deletes" files the user picked while browsing a folder — in keeping
/// with ADR 0002, this is a `Move` into the configured trash folder at the
/// top of `root` (keeping each file's path relative to `root`), not a real
/// delete. That's the same one-trash-per-root layout Dedup uses, so Browse's
/// Empty Trash finds everything deleted here. It shows up in History and can
/// be undone exactly like a Dedup run.
///
/// Refuses a protected `root` (drive roots, core OS folders) and any path
/// that isn't inside `root`, before touching anything.
pub fn delete_files_at(
    root: &Path,
    paths: &[PathBuf],
    data_dir: &Path,
    trash_folder_name: &str,
) -> Result<RunRecord, AppError> {
    if scanner::is_protected_root(root) {
        return Err(AppError::ProtectedPath(root.to_path_buf()));
    }

    let mut operations = Vec::with_capacity(paths.len());
    for source in paths {
        if source == root || !source.starts_with(root) {
            return Err(AppError::Other(format!(
                "{} isn't inside the folder being browsed ({})",
                source.display(),
                root.display()
            )));
        }
        let metadata = std::fs::metadata(source).map_err(|e| AppError::io(source.clone(), e))?;
        if !metadata.is_file() {
            return Err(AppError::Other(format!("not a file: {}", source.display())));
        }

        operations.push(Operation::new(
            OperationKind::MoveToTrash,
            source.clone(),
            staged_destination(root, source, trash_folder_name),
            "Deleted from folder browser",
            metadata.len(),
        ));
    }

    let selected_ids: Vec<String> = operations.iter().map(|op| op.id.clone()).collect();
    let plan = Plan::new(root.to_path_buf(), PlanMode::Delete, operations);
    let record = executor::apply(&plan, &selected_ids);

    store::save_run(data_dir, &record)?;
    Ok(record)
}

#[tauri::command]
pub async fn delete_files(
    app: AppHandle,
    root: String,
    paths: Vec<String>,
) -> Result<RunRecord, AppError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    let trash = load_trash_settings(&app)?;
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    blocking(move || {
        delete_files_at(
            Path::new(&root),
            &paths,
            &data_dir,
            &trash.staging_folder_name,
        )
    })
    .await
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::AppSettings;
    use std::fs;

    #[test]
    fn browse_folder_at_lists_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested/b.txt"), b"b").unwrap();

        let entries = browse_folder_at(dir.path(), &AppSettings::default().trash).unwrap();

        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn browse_folder_at_excludes_trash_and_archive_folders() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::create_dir(dir.path().join(".sortty-trash")).unwrap();
        fs::write(dir.path().join(".sortty-trash/gone.txt"), b"gone").unwrap();
        fs::create_dir(dir.path().join(".sortty-archive")).unwrap();
        fs::write(dir.path().join(".sortty-archive/old.txt"), b"old").unwrap();

        let entries = browse_folder_at(dir.path(), &AppSettings::default().trash).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "a.txt");
    }

    fn delete(root: &Path, paths: &[PathBuf], data_dir: &Path) -> Result<RunRecord, AppError> {
        delete_files_at(root, paths, data_dir, ".sortty-trash")
    }

    #[test]
    fn moves_files_into_the_roots_trash_keeping_relative_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/b.txt"), b"b").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let record = delete(
            root,
            &[root.join("a.txt"), root.join("nested/b.txt")],
            data_dir.path(),
        )
        .unwrap();

        assert_eq!(record.applied_operations.len(), 2);
        assert!(root.join(".sortty-trash/a.txt").exists());
        // Regression: this used to land in nested/.sortty-trash, where
        // Empty Trash (which only looks at the root) never found it.
        assert!(root.join(".sortty-trash/nested/b.txt").exists());
        assert!(!root.join("nested/.sortty-trash").exists());
    }

    #[test]
    fn uses_the_configured_trash_folder_name() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        delete_files_at(root, &[root.join("a.txt")], data_dir.path(), "MyTrash").unwrap();

        assert!(root.join("MyTrash/a.txt").exists());
    }

    #[test]
    fn refuses_a_path_outside_the_root_before_touching_anything() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("inside.txt"), b"a").unwrap();
        fs::write(dir.path().join("outside.txt"), b"b").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let err = delete(
            &root,
            &[root.join("inside.txt"), dir.path().join("outside.txt")],
            data_dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Other(_)));
        assert!(root.join("inside.txt").exists());
        assert!(dir.path().join("outside.txt").exists());
    }

    #[test]
    fn errors_on_first_missing_file_without_touching_files_ordered_after_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("real.txt"), b"a").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let err = delete(
            root,
            &[root.join("missing.txt"), root.join("real.txt")],
            data_dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Io { .. }));
        assert!(root.join("real.txt").exists());
    }

    #[test]
    fn persists_run_with_delete_mode_that_round_trips_via_store() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let record = delete(root, &[root.join("a.txt")], data_dir.path()).unwrap();

        let fetched = store::get_run(data_dir.path(), &record.run_id).unwrap();
        assert_eq!(fetched.plan_mode, PlanMode::Delete);
        assert_eq!(fetched.applied_operations.len(), 1);
    }

    #[test]
    fn refuses_a_protected_root() {
        let data_dir = tempfile::tempdir().unwrap();
        let err = delete(
            Path::new("C:\\"),
            &[PathBuf::from("C:\\x.txt")],
            data_dir.path(),
        )
        .unwrap_err();
        assert!(matches!(err, AppError::ProtectedPath(_)));
    }

    #[test]
    fn refuses_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let err = delete(dir.path(), &[dir.path().join("sub")], data_dir.path()).unwrap_err();

        assert!(matches!(err, AppError::Other(_)));
    }
}
