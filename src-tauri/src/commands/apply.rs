use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::apply::{executor, store};
use crate::commands::cancel::CancelFlag;
use crate::domain::plan::Plan;
use crate::domain::run::RunRecord;
use crate::error::AppError;

#[derive(Clone, Serialize)]
pub struct ApplyProgress {
    pub completed: usize,
    pub total: usize,
}

/// Cancelling mid-apply stops before the next operation's move (never
/// mid-move) and returns a normal `Ok(RunRecord)` with `cancelled: true` and
/// whatever was actually applied so far — not an error, since a partial
/// apply is a real, useful result, not a failure.
#[tauri::command]
pub async fn apply_plan(
    app: AppHandle,
    cancel_flag: State<'_, CancelFlag>,
    plan: Plan,
    selected_ids: Vec<String>,
    on_progress: Channel<ApplyProgress>,
) -> Result<RunRecord, AppError> {
    cancel_flag.reset();
    let mut last_sent = 0usize;
    let record = executor::apply_with_progress(
        &plan,
        &selected_ids,
        |completed, total| {
            if completed - last_sent >= 50 || completed == total {
                on_progress.send(ApplyProgress { completed, total }).ok();
                last_sent = completed;
            }
        },
        || cancel_flag.is_cancelled(),
    );
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    store::save_run(&data_dir, &record)?;
    Ok(record)
}
