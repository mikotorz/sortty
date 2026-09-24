use serde::Serialize;
use std::path::Path;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::apply::{executor, store};
use crate::commands::blocking;
use crate::commands::cancel::CancelFlag;
use crate::commands::history::prune_history;
use crate::commands::plan::PlanStore;
use crate::domain::run::RunRecord;
use crate::error::AppError;

#[derive(Clone, Serialize)]
pub struct ApplyProgress {
    pub completed: usize,
    pub total: usize,
}

/// Applies the selected operations of the plan stored under `plan_id` and
/// saves the run. The plan comes from the backend's own `PlanStore`, never
/// from the caller, so only moves the backend itself planned can run
/// (ADR 0016). The plan is taken out of the store, so it can't be applied
/// twice.
pub fn apply_plan_at(
    plans: &PlanStore,
    plan_id: &str,
    selected_ids: &[String],
    data_dir: &Path,
    on_progress: impl FnMut(usize, usize),
    should_cancel: impl Fn() -> bool,
) -> Result<RunRecord, AppError> {
    let plan = plans.take(plan_id)?;
    let record = executor::apply_with_progress(&plan, selected_ids, on_progress, should_cancel);
    store::save_run(data_dir, &record)?;
    Ok(record)
}

/// Cancelling mid-apply stops before the next operation's move (never
/// mid-move) and returns a normal `Ok(RunRecord)` with `cancelled: true` and
/// whatever was actually applied so far — not an error, since a partial
/// apply is a real, useful result, not a failure.
#[tauri::command]
pub async fn apply_plan(
    app: AppHandle,
    cancel_flag: State<'_, CancelFlag>,
    plan_store: State<'_, PlanStore>,
    plan_id: String,
    selected_ids: Vec<String>,
    on_progress: Channel<ApplyProgress>,
) -> Result<RunRecord, AppError> {
    cancel_flag.reset();
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    let cancel_flag = cancel_flag.inner().clone();
    let plan_store = plan_store.inner().clone();
    blocking(move || {
        let mut last_sent = 0usize;
        let record = apply_plan_at(
            &plan_store,
            &plan_id,
            &selected_ids,
            &data_dir,
            |completed, total| {
                if completed - last_sent >= 50 || completed == total {
                    on_progress.send(ApplyProgress { completed, total }).ok();
                    last_sent = completed;
                }
            },
            || cancel_flag.is_cancelled(),
        )?;
        prune_history(&config_dir, &data_dir);
        Ok(record)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{Operation, OperationKind, Plan, PlanMode};
    use std::fs;

    fn stored_plan(root: &Path, plans: &PlanStore) -> Plan {
        fs::write(root.join("a.txt"), b"a").unwrap();
        let op = Operation::new(
            OperationKind::Move,
            root.join("a.txt"),
            root.join("Docs/a.txt"),
            "test",
            1,
        );
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op]);
        plans.put(plan.clone());
        plan
    }

    #[test]
    fn applies_the_stored_plan_and_saves_the_run() {
        let dir = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let plans = PlanStore::default();
        let plan = stored_plan(dir.path(), &plans);
        let ids = vec![plan.operations[0].id.clone()];

        let record =
            apply_plan_at(&plans, &plan.id, &ids, data.path(), |_, _| {}, || false).unwrap();

        assert_eq!(record.applied_operations.len(), 1);
        assert!(dir.path().join("Docs/a.txt").exists());
        assert!(store::get_run(data.path(), &record.run_id).is_ok());
    }

    #[test]
    fn refuses_an_unknown_plan_id_without_moving_anything() {
        let dir = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let plans = PlanStore::default();
        let plan = stored_plan(dir.path(), &plans);
        let ids = vec![plan.operations[0].id.clone()];

        let err =
            apply_plan_at(&plans, "stale-id", &ids, data.path(), |_, _| {}, || false).unwrap_err();

        assert!(matches!(err, AppError::InvalidPlan(_)));
        assert!(dir.path().join("a.txt").exists());
    }

    #[test]
    fn a_plan_can_only_be_applied_once() {
        let dir = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let plans = PlanStore::default();
        let plan = stored_plan(dir.path(), &plans);
        let ids = vec![plan.operations[0].id.clone()];

        apply_plan_at(&plans, &plan.id, &ids, data.path(), |_, _| {}, || false).unwrap();
        let second = apply_plan_at(&plans, &plan.id, &ids, data.path(), |_, _| {}, || false);

        assert!(matches!(second, Err(AppError::InvalidPlan(_))));
    }
}
