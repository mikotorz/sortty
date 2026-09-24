use chrono::{DateTime, Duration, Utc};
use std::path::{Path, PathBuf};

use crate::domain::run::{RunRecord, RunSummary};
use crate::error::AppError;
use crate::fsutil::write_atomic;

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
    write_atomic(&path, &text)
}

pub fn save_run(app_data_dir: &Path, record: &RunRecord) -> Result<(), AppError> {
    let dir = runs_dir(app_data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::io(dir.clone(), e))?;

    let path = run_path(app_data_dir, &record.run_id);
    let text = serde_json::to_string_pretty(record)
        .map_err(|e| AppError::Config(format!("failed to serialize run record: {e}")))?;
    write_atomic(&path, &text)?;

    let mut index = read_index(app_data_dir)?;
    index.retain(|s| s.run_id != record.run_id);
    index.insert(0, RunSummary::from(record));
    write_index(app_data_dir, &index)?;

    Ok(())
}

pub fn list_runs(app_data_dir: &Path, limit: Option<usize>) -> Result<Vec<RunSummary>, AppError> {
    let mut index = read_index(app_data_dir)?;
    index.sort_by_key(|s| std::cmp::Reverse(s.started_at));
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

/// History always keeps at least this many of the newest runs, however old
/// they are (ADR 0020).
pub const MIN_KEPT_RUNS: usize = 200;

/// Deletes runs that are **both** outside the newest [`MIN_KEPT_RUNS`] **and**
/// started more than `keep_days` days before `now`. `keep_days == 0` keeps
/// everything. Returns how many runs were removed. A pruned run can no longer
/// be undone, which is why the rule needs both conditions.
pub fn prune_runs(
    app_data_dir: &Path,
    keep_days: u32,
    now: DateTime<Utc>,
) -> Result<usize, AppError> {
    if keep_days == 0 {
        return Ok(0);
    }
    let mut index = read_index(app_data_dir)?;
    if index.len() <= MIN_KEPT_RUNS {
        return Ok(0);
    }
    index.sort_by_key(|s| std::cmp::Reverse(s.started_at));
    let cutoff = now - Duration::days(i64::from(keep_days));

    let mut kept = Vec::with_capacity(index.len());
    let mut pruned = 0;
    for (position, summary) in index.into_iter().enumerate() {
        if position < MIN_KEPT_RUNS || summary.started_at >= cutoff {
            kept.push(summary);
            continue;
        }
        let path = run_path(app_data_dir, &summary.run_id);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AppError::io(path, e)),
        }
        pruned += 1;
    }
    if pruned > 0 {
        write_index(app_data_dir, &kept)?;
    }
    Ok(pruned)
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

    fn sample_record(run_id: &str) -> RunRecord {
        sample_record_at(run_id, Utc::now())
    }

    fn sample_record_at(run_id: &str, started_at: DateTime<Utc>) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            root: PathBuf::from("/root"),
            plan_mode: PlanMode::SortByType,
            started_at,
            finished_at: Utc::now(),
            applied_operations: vec![],
            failed_operations: vec![],
            undone: false,
            cancelled: false,
            restored_ids: vec![],
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

    /// Saves `MIN_KEPT_RUNS` recent runs, then one more run per entry of
    /// `extra_ages_days`, started that many days before `now`.
    fn history_with(dir: &Path, now: DateTime<Utc>, extra_ages_days: &[i64]) {
        for i in 0..MIN_KEPT_RUNS {
            let at = now - Duration::minutes(i as i64);
            save_run(dir, &sample_record_at(&format!("recent-{i}"), at)).unwrap();
        }
        for (i, age) in extra_ages_days.iter().enumerate() {
            let at = now - Duration::days(*age);
            save_run(dir, &sample_record_at(&format!("extra-{i}"), at)).unwrap();
        }
    }

    #[test]
    fn prunes_only_old_runs_beyond_the_newest_200() {
        let dir = tempfile::tempdir().unwrap();
        let now = Utc::now();
        // extra-0 is recent (kept despite being past #200); extra-1 is old (pruned).
        history_with(dir.path(), now, &[10, 120]);

        assert_eq!(prune_runs(dir.path(), 90, now).unwrap(), 1);

        let ids: Vec<String> = list_runs(dir.path(), None)
            .unwrap()
            .into_iter()
            .map(|s| s.run_id)
            .collect();
        assert_eq!(ids.len(), MIN_KEPT_RUNS + 1);
        assert!(ids.contains(&"extra-0".to_string()));
        assert!(!ids.contains(&"extra-1".to_string()));
        assert!(matches!(
            get_run(dir.path(), "extra-1"),
            Err(AppError::RunNotFound(_))
        ));
    }

    #[test]
    fn keeps_old_runs_within_the_newest_200() {
        let dir = tempfile::tempdir().unwrap();
        let now = Utc::now();
        let ancient = sample_record_at("ancient", now - Duration::days(1000));
        save_run(dir.path(), &ancient).unwrap();
        assert_eq!(prune_runs(dir.path(), 90, now).unwrap(), 0);
        assert!(get_run(dir.path(), "ancient").is_ok());
    }

    #[test]
    fn zero_days_keeps_everything() {
        let dir = tempfile::tempdir().unwrap();
        let now = Utc::now();
        history_with(dir.path(), now, &[1000]);
        assert_eq!(prune_runs(dir.path(), 0, now).unwrap(), 0);
        assert_eq!(
            list_runs(dir.path(), None).unwrap().len(),
            MIN_KEPT_RUNS + 1
        );
    }
}
