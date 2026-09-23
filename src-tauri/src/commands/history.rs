use tauri::{AppHandle, Manager};

use crate::apply::{store, undo};
use crate::domain::run::{RunRecord, RunSummary, UndoResult};
use crate::error::AppError;

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
pub async fn list_runs(app: AppHandle, limit: Option<usize>) -> Result<Vec<RunSummary>, AppError> {
    store::list_runs(&data_dir(&app)?, limit)
}

#[tauri::command]
pub async fn get_run(app: AppHandle, run_id: String) -> Result<RunRecord, AppError> {
    store::get_run(&data_dir(&app)?, &run_id)
}

#[tauri::command]
pub async fn undo_run(app: AppHandle, run_id: String) -> Result<UndoResult, AppError> {
    let dir = data_dir(&app)?;
    let record = store::get_run(&dir, &run_id)?;
    if record.undone {
        return Err(AppError::InvalidPlan(format!(
            "run {run_id} was already undone"
        )));
    }
    let result = undo::undo(&record)?;
    store::mark_undone(&dir, &run_id)?;
    Ok(result)
}

#[tauri::command]
pub async fn undo_last_run(app: AppHandle) -> Result<UndoResult, AppError> {
    let dir = data_dir(&app)?;
    let runs = store::list_runs(&dir, None)?;
    let last = runs
        .into_iter()
        .find(|r| !r.undone)
        .ok_or_else(|| AppError::RunNotFound("no undoable runs".to_string()))?;
    let record = store::get_run(&dir, &last.run_id)?;
    let result = undo::undo(&record)?;
    store::mark_undone(&dir, &last.run_id)?;
    Ok(result)
}
