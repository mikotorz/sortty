use std::path::{Path, PathBuf};

use crate::domain::run::{RunRecord, RunSummary};
use crate::error::AppError;

fn runs_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("runs")
}

fn run_path(app_data_dir: &Path, run_id: &str) -> PathBuf {
    runs_dir(app_data_dir).join(format!("{run_id}.json"))
}

fn index_path(app_data_dir: &Path) -> PathBuf {
    runs_dir(app_data_dir).join("index.json")
}

fn read_index(app_data_dir: &Path) -> Result<Vec<RunSummary>, AppError> {
    let path = index_path(app_data_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| AppError::io(path.clone(), e))?;
    serde_json::from_str(&text).map_err(|e| AppError::Config(format!("run index: {e}")))
}

fn write_index(app_data_dir: &Path, index: &[RunSummary]) -> Result<(), AppError> {
    let path = index_path(app_data_dir);
    let text = serde_json::to_string_pretty(index)
        .map_err(|e| AppError::Config(format!("failed to serialize run index: {e}")))?;
    std::fs::write(&path, text).map_err(|e| AppError::io(path, e))
}

pub fn save_run(app_data_dir: &Path, record: &RunRecord) -> Result<(), AppError> {
    let dir = runs_dir(app_data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::io(dir.clone(), e))?;

    let path = run_path(app_data_dir, &record.run_id);
    let text = serde_json::to_string_pretty(record)
        .map_err(|e| AppError::Config(format!("failed to serialize run record: {e}")))?;
    std::fs::write(&path, text).map_err(|e| AppError::io(path, e))?;

    let mut index = read_index(app_data_dir)?;
    index.retain(|s| s.run_id != record.run_id);
    index.insert(0, RunSummary::from(record));
    write_index(app_data_dir, &index)?;

    Ok(())
}

pub fn list_runs(app_data_dir: &Path, limit: Option<usize>) -> Result<Vec<RunSummary>, AppError> {
    let mut index = read_index(app_data_dir)?;
    index.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    if let Some(limit) = limit {
        index.truncate(limit);
    }
    Ok(index)
}

pub fn get_run(app_data_dir: &Path, run_id: &str) -> Result<RunRecord, AppError> {
    let path = run_path(app_data_dir, run_id);
    if !path.exists() {
        return Err(AppError::RunNotFound(run_id.to_string()));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| AppError::io(path.clone(), e))?;
    serde_json::from_str(&text).map_err(|e| AppError::Config(format!("run record: {e}")))
}

pub fn mark_undone(app_data_dir: &Path, run_id: &str) -> Result<(), AppError> {
    let mut record = get_run(app_data_dir, run_id)?;
    record.undone = true;
    save_run(app_data_dir, &record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::PlanMode;
    use chrono::Utc;

    fn sample_record(run_id: &str) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            root: PathBuf::from("/root"),
            plan_mode: PlanMode::SortByType,
            started_at: Utc::now(),
            finished_at: Utc::now(),
            applied_operations: vec![],
            failed_operations: vec![],
            undone: false,
        }
    }

    #[test]
    fn saves_and_retrieves_a_run() {
        let dir = tempfile::tempdir().unwrap();
        let record = sample_record("run-1");
        save_run(dir.path(), &record).unwrap();

        let fetched = get_run(dir.path(), "run-1").unwrap();
        assert_eq!(fetched.run_id, "run-1");

        let summaries = list_runs(dir.path(), None).unwrap();
        assert_eq!(summaries.len(), 1);
    }

    #[test]
    fn mark_undone_persists() {
        let dir = tempfile::tempdir().unwrap();
        save_run(dir.path(), &sample_record("run-1")).unwrap();
        mark_undone(dir.path(), "run-1").unwrap();
        assert!(get_run(dir.path(), "run-1").unwrap().undone);
    }
}
