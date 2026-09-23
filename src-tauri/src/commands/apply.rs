use tauri::{AppHandle, Manager};

use crate::apply::{executor, store};
use crate::domain::plan::Plan;
use crate::domain::run::RunRecord;
use crate::error::AppError;

#[tauri::command]
pub async fn apply_plan(
    app: AppHandle,
    plan: Plan,
    selected_ids: Vec<String>,
) -> Result<RunRecord, AppError> {
    let record = executor::apply(&plan, &selected_ids);
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    store::save_run(&data_dir, &record)?;
    Ok(record)
}
