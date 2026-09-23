use chrono::Utc;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::plan::Plan;
use crate::domain::run::{AppliedOperation, FailedOperation, RunRecord};
use crate::fsutil::move_no_clobber;

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
    let ext = destination
        .extension()
        .map(|e| e.to_string_lossy().to_string());

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

pub fn apply(plan: &Plan, selected_ids: &[String]) -> RunRecord {
    apply_with_progress(plan, selected_ids, |_, _| {}, || false)
}

/// Same as `apply`, but reports `(completed, total)` selected-operation
/// counts via `on_progress` (the total is known upfront, unlike a scan) and
/// checks `should_cancel` before starting each operation's move — never
/// mid-move, so nothing is ever left half-moved. On cancellation, the loop
/// stops and the returned `RunRecord` has `cancelled: true`, with
/// `applied_operations` holding whatever was actually moved so far.
pub fn apply_with_progress(
    plan: &Plan,
    selected_ids: &[String],
    mut on_progress: impl FnMut(usize, usize),
    should_cancel: impl Fn() -> bool,
) -> RunRecord {
    let started_at = Utc::now();
    let selected: HashSet<&str> = selected_ids.iter().map(String::as_str).collect();
    let total = selected.len();

    let mut applied = Vec::new();
    let mut failed = Vec::new();
    let mut cancelled = false;
    let mut completed = 0usize;

    for op in &plan.operations {
        if !selected.contains(op.id.as_str()) {
            continue;
        }

        if should_cancel() {
            cancelled = true;
            break;
        }

        let final_destination = resolve_collision(&op.destination);
        match move_no_clobber(&op.source, &final_destination) {
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
        completed += 1;
        on_progress(completed, total);
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
        cancelled,
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
        let plan = Plan::new(
            root.to_path_buf(),
            PlanMode::SortByType,
            vec![op_a.clone(), op_b],
        );

        let record = apply(&plan, std::slice::from_ref(&op_a.id));

        assert_eq!(record.applied_operations.len(), 1);
        assert!(root.join("Docs/a.txt").exists());
        assert!(root.join("b.txt").exists()); // untouched
        assert!(!root.join("Docs/b.txt").exists());
    }

    #[test]
    fn apply_with_progress_reports_completed_of_total() {
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
        let ids = vec![op_a.id.clone(), op_b.id.clone()];
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op_a, op_b]);

        let mut progress = Vec::new();
        let record = apply_with_progress(
            &plan,
            &ids,
            |completed, total| progress.push((completed, total)),
            || false,
        );

        assert!(!record.cancelled);
        assert_eq!(record.applied_operations.len(), 2);
        assert_eq!(progress, vec![(1, 2), (2, 2)]);
    }

    #[test]
    fn apply_with_progress_stops_before_the_next_move_when_cancelled() {
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
        let ids = vec![op_a.id.clone(), op_b.id.clone()];
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op_a, op_b]);

        let record = apply_with_progress(&plan, &ids, |_, _| {}, || true);

        assert!(record.cancelled);
        assert!(record.applied_operations.is_empty());
        assert!(root.join("a.txt").exists());
        assert!(root.join("b.txt").exists());
    }

    /// Regression test for the architecture review's long-path (MAX_PATH)
    /// concern: no `\\?\` prefixing exists anywhere in this module, so this
    /// proves `move_file` (via `create_dir_all` + `rename`) already succeeds
    /// for a destination well past Windows' legacy 260-character limit on
    /// the toolchain this app builds with. If this test ever starts failing,
    /// that's the trigger to add explicit long-path prefixing — not before.
    #[test]
    fn moves_file_to_a_destination_path_over_260_chars() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("f.txt"), b"x").unwrap();

        let long_segment = "a".repeat(50);
        let mut dest_dir = root.to_path_buf();
        for _ in 0..5 {
            dest_dir = dest_dir.join(&long_segment);
        }
        let destination = dest_dir.join("f.txt");
        assert!(
            destination.as_os_str().len() > 260,
            "test setup should itself exceed MAX_PATH"
        );

        let op = Operation::new(
            OperationKind::Move,
            root.join("f.txt"),
            destination.clone(),
            "test",
            1,
        );
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op.clone()]);
        let record = apply(&plan, std::slice::from_ref(&op.id));

        assert_eq!(record.applied_operations.len(), 1);
        assert!(record.failed_operations.is_empty());
        assert!(destination.exists());
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
        let record = apply(&plan, std::slice::from_ref(&op.id));

        assert_eq!(record.applied_operations.len(), 1);
        assert_eq!(
            fs::read_to_string(root.join("Docs/a.txt")).unwrap(),
            "existing"
        );
        assert_eq!(
            fs::read_to_string(root.join("Docs/a (1).txt")).unwrap(),
            "incoming"
        );
    }
}
