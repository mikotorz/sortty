use std::path::Path;
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

pub fn undo_run_at(data_dir: &Path, run_id: &str) -> Result<UndoResult, AppError> {
    let record = store::get_run(data_dir, run_id)?;
    if record.undone {
        return Err(AppError::InvalidPlan(format!(
            "run {run_id} was already undone"
        )));
    }
    let result = undo::undo(&record)?;
    store::mark_undone(data_dir, run_id)?;
    Ok(result)
}

#[tauri::command]
pub async fn undo_run(app: AppHandle, run_id: String) -> Result<UndoResult, AppError> {
    undo_run_at(&data_dir(&app)?, &run_id)
}

/// Undoes the most recent run that hasn't already been undone. Relies on
/// `store::list_runs` sorting newest-first — see the "most recent" test
/// coverage below, which pins that cross-module invariant explicitly.
pub fn undo_last_run_at(data_dir: &Path) -> Result<UndoResult, AppError> {
    let runs = store::list_runs(data_dir, None)?;
    let last = runs
        .into_iter()
        .find(|r| !r.undone)
        .ok_or_else(|| AppError::RunNotFound("no undoable runs".to_string()))?;
    let record = store::get_run(data_dir, &last.run_id)?;
    let result = undo::undo(&record)?;
    store::mark_undone(data_dir, &last.run_id)?;
    Ok(result)
}

#[tauri::command]
pub async fn undo_last_run(app: AppHandle) -> Result<UndoResult, AppError> {
    undo_last_run_at(&data_dir(&app)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{OperationKind, PlanMode};
    use crate::domain::run::AppliedOperation;
    use chrono::{Duration, Utc};
    use std::fs;
    use std::path::PathBuf;

    fn make_record(
        run_id: &str,
        started_at: chrono::DateTime<Utc>,
        ops: Vec<AppliedOperation>,
    ) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            root: PathBuf::from("root"),
            plan_mode: PlanMode::SortByType,
            started_at,
            finished_at: started_at,
            applied_operations: ops,
            failed_operations: vec![],
            undone: false,
        }
    }

    fn moved_op(id: &str, from: PathBuf, to: PathBuf) -> AppliedOperation {
        AppliedOperation {
            id: id.to_string(),
            kind: OperationKind::Move,
            from,
            to,
            size_bytes: 1,
        }
    }

    #[test]
    fn undo_last_run_at_undoes_the_most_recent_non_undone_run() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();
        fs::create_dir_all(data_dir.join("Docs")).unwrap();
        fs::write(data_dir.join("Docs/older.txt"), b"older").unwrap();
        fs::write(data_dir.join("Docs/newer.txt"), b"newer").unwrap();

        let older = make_record(
            "run-older",
            Utc::now() - Duration::hours(1),
            vec![moved_op(
                "1",
                data_dir.join("older.txt"),
                data_dir.join("Docs/older.txt"),
            )],
        );
        let newer = make_record(
            "run-newer",
            Utc::now(),
            vec![moved_op(
                "2",
                data_dir.join("newer.txt"),
                data_dir.join("Docs/newer.txt"),
            )],
        );
        store::save_run(data_dir, &older).unwrap();
        store::save_run(data_dir, &newer).unwrap();

        let result = undo_last_run_at(data_dir).unwrap();

        assert_eq!(result.restored, 1);
        assert!(data_dir.join("newer.txt").exists());
        assert!(!data_dir.join("Docs/newer.txt").exists());
        // The older run is untouched.
        assert!(data_dir.join("Docs/older.txt").exists());
        assert!(store::get_run(data_dir, "run-newer").unwrap().undone);
        assert!(!store::get_run(data_dir, "run-older").unwrap().undone);
    }

    #[test]
    fn undo_last_run_at_skips_an_already_undone_newest_run() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();
        fs::create_dir_all(data_dir.join("Docs")).unwrap();
        fs::write(data_dir.join("Docs/older.txt"), b"older").unwrap();
        fs::write(data_dir.join("Docs/newer.txt"), b"newer").unwrap();

        let older = make_record(
            "run-older",
            Utc::now() - Duration::hours(1),
            vec![moved_op(
                "1",
                data_dir.join("older.txt"),
                data_dir.join("Docs/older.txt"),
            )],
        );
        let newer = make_record(
            "run-newer",
            Utc::now(),
            vec![moved_op(
                "2",
                data_dir.join("newer.txt"),
                data_dir.join("Docs/newer.txt"),
            )],
        );
        store::save_run(data_dir, &older).unwrap();
        store::save_run(data_dir, &newer).unwrap();
        store::mark_undone(data_dir, "run-newer").unwrap();

        let result = undo_last_run_at(data_dir).unwrap();

        assert_eq!(result.restored, 1);
        assert!(data_dir.join("older.txt").exists());
        assert!(store::get_run(data_dir, "run-older").unwrap().undone);
    }

    #[test]
    fn undo_last_run_at_errors_when_no_undoable_runs_exist() {
        let dir = tempfile::tempdir().unwrap();
        let err = undo_last_run_at(dir.path()).unwrap_err();
        assert!(matches!(err, AppError::RunNotFound(_)));
    }

    #[test]
    fn undo_run_at_restores_the_given_run_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();
        fs::create_dir_all(data_dir.join("Docs")).unwrap();
        fs::write(data_dir.join("Docs/a.txt"), b"a").unwrap();

        let record = make_record(
            "run-1",
            Utc::now(),
            vec![moved_op(
                "1",
                data_dir.join("a.txt"),
                data_dir.join("Docs/a.txt"),
            )],
        );
        store::save_run(data_dir, &record).unwrap();

        let result = undo_run_at(data_dir, "run-1").unwrap();

        assert_eq!(result.restored, 1);
        assert!(data_dir.join("a.txt").exists());
        assert!(store::get_run(data_dir, "run-1").unwrap().undone);
    }

    #[test]
    fn undo_run_at_errors_when_already_undone() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();
        let record = make_record("run-1", Utc::now(), vec![]);
        store::save_run(data_dir, &record).unwrap();
        store::mark_undone(data_dir, "run-1").unwrap();

        let err = undo_run_at(data_dir, "run-1").unwrap_err();
        assert!(matches!(err, AppError::InvalidPlan(_)));
    }
}
