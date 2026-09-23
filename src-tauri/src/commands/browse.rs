use std::path::PathBuf;
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
#[tauri::command]
pub async fn browse_folder(path: String) -> Result<Vec<FileEntry>, AppError> {
    let folder = PathBuf::from(path);
    let options = ScanOptions {
        include_subfolders: true,
        exclude: vec![".sortty-trash".to_string(), ".sortty-archive".to_string()],
        exclude_folders: Vec::new(),
    };
    scanner::scan(&folder, &options)
}

/// "Deletes" files the user picked while browsing a destination folder — in
/// keeping with ADR 0002, this is a `Move` into a `.sortty-trash` folder next
/// to each file, not a real delete, so it shows up in History and can be
/// undone exactly like a Dedup run.
#[tauri::command]
pub async fn delete_files(
    app: AppHandle,
    root: String,
    paths: Vec<String>,
) -> Result<RunRecord, AppError> {
    let mut operations = Vec::with_capacity(paths.len());
    for path in paths {
        let source = PathBuf::from(path);
        let metadata = std::fs::metadata(&source).map_err(|e| AppError::io(source.clone(), e))?;
        let parent = source.parent().unwrap_or(&source).to_path_buf();
        let file_name = source
            .file_name()
            .ok_or_else(|| AppError::Other(format!("not a file: {}", source.display())))?;
        let destination = parent.join(".sortty-trash").join(file_name);

        operations.push(Operation::new(
            OperationKind::MoveToTrash,
            source,
            destination,
            "Deleted from folder browser",
            metadata.len(),
        ));
    }

    let selected_ids: Vec<String> = operations.iter().map(|op| op.id.clone()).collect();
    let plan = Plan::new(PathBuf::from(root), PlanMode::Delete, operations);
    let record = executor::apply(&plan, &selected_ids);

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    store::save_run(&data_dir, &record)?;
    Ok(record)
}
