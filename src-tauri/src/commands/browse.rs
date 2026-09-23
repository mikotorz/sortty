use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::apply::{executor, store};
use crate::domain::entry::FileEntry;
use crate::domain::plan::{Operation, OperationKind, Plan, PlanMode};
use crate::domain::run::RunRecord;
use crate::engine::scanner::{self, ScanOptions};
use crate::error::AppError;

/// Lists the current contents of an already-sorted destination folder, so the
/// user can pick files to delete. Reuses the ordinary scanner rather than a
/// separate listing routine — a destination folder is scanned the same way
/// any other folder is, just always recursively and always skipping the
/// tool's own staging folders.
pub fn browse_folder_at(folder: &Path) -> Result<Vec<FileEntry>, AppError> {
    let options = ScanOptions {
        include_subfolders: true,
        exclude: vec![".sortty-trash".to_string(), ".sortty-archive".to_string()],
        exclude_folders: Vec::new(),
    };
    scanner::scan(folder, &options)
}

#[tauri::command]
pub async fn browse_folder(path: String) -> Result<Vec<FileEntry>, AppError> {
    browse_folder_at(Path::new(&path))
}

/// "Deletes" files the user picked while browsing a destination folder — in
/// keeping with ADR 0002, this is a `Move` into a `.sortty-trash` folder next
/// to each file, not a real delete, so it shows up in History and can be
/// undone exactly like a Dedup run.
pub fn delete_files_at(
    root: &Path,
    paths: &[PathBuf],
    data_dir: &Path,
) -> Result<RunRecord, AppError> {
    let mut operations = Vec::with_capacity(paths.len());
    for source in paths {
        let metadata = std::fs::metadata(source).map_err(|e| AppError::io(source.clone(), e))?;
        let parent = source.parent().unwrap_or(source).to_path_buf();
        let file_name = source
            .file_name()
            .ok_or_else(|| AppError::Other(format!("not a file: {}", source.display())))?;
        let destination = parent.join(".sortty-trash").join(file_name);

        operations.push(Operation::new(
            OperationKind::MoveToTrash,
            source.clone(),
            destination,
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
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    delete_files_at(Path::new(&root), &paths, &data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn browse_folder_at_lists_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested/b.txt"), b"b").unwrap();

        let entries = browse_folder_at(dir.path()).unwrap();

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

        let entries = browse_folder_at(dir.path()).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "a.txt");
    }

    #[test]
    fn moves_files_into_per_parent_sortty_trash_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        fs::write(root.join("b.txt"), b"b").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let record = delete_files_at(
            root,
            &[root.join("a.txt"), root.join("b.txt")],
            data_dir.path(),
        )
        .unwrap();

        assert_eq!(record.applied_operations.len(), 2);
        assert!(!root.join("a.txt").exists());
        assert!(!root.join("b.txt").exists());
        assert!(root.join(".sortty-trash/a.txt").exists());
        assert!(root.join(".sortty-trash/b.txt").exists());
    }

    #[test]
    fn errors_on_first_missing_file_without_touching_files_ordered_after_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("real.txt"), b"a").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let err = delete_files_at(
            root,
            &[root.join("missing.txt"), root.join("real.txt")],
            data_dir.path(),
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Io { .. }));
        // Documents current behavior: the whole call fails before any file
        // is touched once one input path can't be stat'd.
        assert!(root.join("real.txt").exists());
    }

    #[test]
    fn persists_run_with_delete_mode_that_round_trips_via_store() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        let data_dir = tempfile::tempdir().unwrap();

        let record = delete_files_at(root, &[root.join("a.txt")], data_dir.path()).unwrap();

        let fetched = store::get_run(data_dir.path(), &record.run_id).unwrap();
        assert_eq!(fetched.plan_mode, PlanMode::Delete);
        assert_eq!(fetched.applied_operations.len(), 1);
    }

    #[test]
    fn errors_on_a_path_with_no_file_name() {
        let data_dir = tempfile::tempdir().unwrap();

        // A drive root exists and has readable metadata, but `file_name()`
        // is `None` for it — the case the explicit check guards against.
        let err = delete_files_at(Path::new("C:\\"), &[PathBuf::from("C:\\")], data_dir.path())
            .unwrap_err();

        assert!(matches!(err, AppError::Other(_)));
    }
}
