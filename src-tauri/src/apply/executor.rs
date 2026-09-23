use chrono::Utc;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::plan::Plan;
use crate::domain::run::{AppliedOperation, FailedOperation, RunRecord};
use crate::error::AppError;

/// If `destination` already exists, append " (1)", " (2)", ... before the
/// extension until a free path is found.
fn resolve_collision(destination: &Path) -> PathBuf {
    if !destination.exists() {
        return destination.to_path_buf();
    }

    let parent = destination.parent().unwrap_or_else(|| Path::new(""));
    let stem = destination
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = destination.extension().map(|e| e.to_string_lossy().to_string());

    let mut n = 1;
    loop {
        let candidate_name = match &ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

fn move_file(from: &Path, to: &Path) -> Result<(), AppError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent.to_path_buf(), e))?;
    }
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        // rename fails across drives/volumes; fall back to copy + remove.
        Err(_) => {
            std::fs::copy(from, to).map_err(|e| AppError::io(from.to_path_buf(), e))?;
            std::fs::remove_file(from).map_err(|e| AppError::io(from.to_path_buf(), e))?;
            Ok(())
        }
    }
}

pub fn apply(plan: &Plan, selected_ids: &[String]) -> RunRecord {
    let started_at = Utc::now();
    let selected: HashSet<&str> = selected_ids.iter().map(String::as_str).collect();

    let mut applied = Vec::new();
    let mut failed = Vec::new();

    for op in &plan.operations {
        if !selected.contains(op.id.as_str()) {
            continue;
        }

        let final_destination = resolve_collision(&op.destination);
        match move_file(&op.source, &final_destination) {
            Ok(()) => applied.push(AppliedOperation {
                id: op.id.clone(),
                kind: op.kind,
                from: op.source.clone(),
                to: final_destination,
                size_bytes: op.size_bytes,
            }),
            Err(e) => failed.push(FailedOperation {
                operation: op.clone(),
                error: e.to_string(),
            }),
        }
    }

    RunRecord {
        run_id: uuid::Uuid::new_v4().to_string(),
        root: plan.root.clone(),
        plan_mode: plan.mode,
        started_at,
        finished_at: Utc::now(),
        applied_operations: applied,
        failed_operations: failed,
        undone: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{Operation, OperationKind, PlanMode};
    use std::fs;

    #[test]
    fn moves_selected_files_and_skips_unselected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        fs::write(root.join("b.txt"), b"b").unwrap();

        let op_a = Operation::new(
            OperationKind::Move,
            root.join("a.txt"),
            root.join("Docs/a.txt"),
            "test",
            1,
        );
        let op_b = Operation::new(
            OperationKind::Move,
            root.join("b.txt"),
            root.join("Docs/b.txt"),
            "test",
            1,
        );
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op_a.clone(), op_b]);

        let record = apply(&plan, &[op_a.id.clone()]);

        assert_eq!(record.applied_operations.len(), 1);
        assert!(root.join("Docs/a.txt").exists());
        assert!(root.join("b.txt").exists()); // untouched
        assert!(!root.join("Docs/b.txt").exists());
    }

    #[test]
    fn resolves_name_collisions_instead_of_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Docs")).unwrap();
        fs::write(root.join("Docs/a.txt"), b"existing").unwrap();
        fs::write(root.join("a.txt"), b"incoming").unwrap();

        let op = Operation::new(
            OperationKind::Move,
            root.join("a.txt"),
            root.join("Docs/a.txt"),
            "test",
            1,
        );
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op.clone()]);
        let record = apply(&plan, &[op.id.clone()]);

        assert_eq!(record.applied_operations.len(), 1);
        assert_eq!(fs::read_to_string(root.join("Docs/a.txt")).unwrap(), "existing");
        assert_eq!(
            fs::read_to_string(root.join("Docs/a (1).txt")).unwrap(),
            "incoming"
        );
    }
}
