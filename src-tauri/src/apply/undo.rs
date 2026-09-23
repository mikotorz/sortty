use std::path::Path;

use crate::domain::run::{RunRecord, UndoResult};
use crate::error::AppError;
use crate::fsutil::move_no_clobber;

/// Reverses a run's applied operations (`to -> from`), most recent first,
/// skipping any already listed in `record.restored_ids` by an earlier,
/// partial undo. A destination that's occupied again (something new landed
/// there since the run) is skipped and reported as a conflict rather than
/// aborting the undo.
///
/// After each restore, folders the run left empty (`Images/`, `2026/01/`,
/// `.sortty-trash/...`) are removed, up to but never including `record.root`.
pub fn undo(record: &RunRecord) -> Result<UndoResult, AppError> {
    let mut result = UndoResult::default();

    for applied in record.applied_operations.iter().rev() {
        if record.restored_ids.contains(&applied.id) {
            continue;
        }
        if !applied.to.exists() {
            // Already moved/removed by something else; nothing to restore.
            result.conflicts.push(applied.clone());
            continue;
        }
        if applied.from.exists() {
            // Something already occupies the original path.
            result.conflicts.push(applied.clone());
            continue;
        }

        match move_no_clobber(&applied.to, &applied.from) {
            Ok(()) => {
                result.restored += 1;
                result.restored_ids.push(applied.id.clone());
                if let Some(parent) = applied.to.parent() {
                    remove_empty_folders(parent, &record.root);
                }
            }
            Err(e) => {
                log::warn!("sortty: couldn't restore {}: {e}", applied.from.display());
                result.conflicts.push(applied.clone());
            }
        }
    }
    Ok(result)
}

/// Removes `dir` and then each of its parents while they're empty, stopping
/// at (and never removing) `root`. `remove_dir` refuses a non-empty folder,
/// so this can never delete a file (ADR 0002).
fn remove_empty_folders(dir: &Path, root: &Path) {
    let mut current = Some(dir);
    while let Some(dir) = current {
        if dir == root || !dir.starts_with(root) || std::fs::remove_dir(dir).is_err() {
            break;
        }
        current = dir.parent();
    }
}

/// Undoes `record` and persists the outcome: newly restored ids are added
/// to `restored_ids`, and `undone` is set only once every applied
/// operation has been restored, so a partial undo can be retried.
pub fn undo_and_record(record: &mut RunRecord) -> Result<UndoResult, AppError> {
    let result = undo(record)?;
    record
        .restored_ids
        .extend(result.restored_ids.iter().cloned());
    record.undone = record
        .applied_operations
        .iter()
        .all(|op| record.restored_ids.contains(&op.id));
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{OperationKind, PlanMode};
    use crate::domain::run::AppliedOperation;
    use chrono::Utc;
    use std::fs;

    fn make_record(root: &std::path::Path, ops: Vec<AppliedOperation>) -> RunRecord {
        RunRecord {
            run_id: "test-run".into(),
            root: root.to_path_buf(),
            plan_mode: PlanMode::SortByType,
            started_at: Utc::now(),
            finished_at: Utc::now(),
            applied_operations: ops,
            failed_operations: vec![],
            undone: false,
            cancelled: false,
            restored_ids: vec![],
        }
    }

    #[test]
    fn restores_moved_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Docs")).unwrap();
        fs::write(root.join("Docs/a.txt"), b"a").unwrap();

        let record = make_record(
            root,
            vec![AppliedOperation {
                id: "1".into(),
                kind: OperationKind::Move,
                from: root.join("a.txt"),
                to: root.join("Docs/a.txt"),
                size_bytes: 1,
            }],
        );

        let result = undo(&record).unwrap();
        assert_eq!(result.restored, 1);
        assert!(result.conflicts.is_empty());
        assert!(root.join("a.txt").exists());
        assert!(!root.join("Docs/a.txt").exists());
    }

    /// Mirrors executor.rs's long-path regression test for the restore
    /// direction: proves `undo` (via `create_dir_all` + `rename`) already
    /// succeeds when the applied operation's `to` path is well past
    /// Windows' legacy 260-character MAX_PATH limit.
    #[test]
    fn restores_a_file_from_a_path_over_260_chars() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let long_segment = "a".repeat(50);
        let mut long_dir = root.to_path_buf();
        for _ in 0..5 {
            long_dir = long_dir.join(&long_segment);
        }
        fs::create_dir_all(&long_dir).unwrap();
        let to = long_dir.join("a.txt");
        fs::write(&to, b"a").unwrap();
        assert!(
            to.as_os_str().len() > 260,
            "test setup should itself exceed MAX_PATH"
        );

        let record = make_record(
            root,
            vec![AppliedOperation {
                id: "1".into(),
                kind: OperationKind::Move,
                from: root.join("a.txt"),
                to: to.clone(),
                size_bytes: 1,
            }],
        );

        let result = undo(&record).unwrap();
        assert_eq!(result.restored, 1);
        assert!(result.conflicts.is_empty());
        assert!(root.join("a.txt").exists());
        assert!(!to.exists());
    }

    #[test]
    fn reports_conflict_when_original_path_is_occupied() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Docs")).unwrap();
        fs::write(root.join("Docs/a.txt"), b"moved").unwrap();
        fs::write(root.join("a.txt"), b"new file since the run").unwrap();

        let record = make_record(
            root,
            vec![AppliedOperation {
                id: "1".into(),
                kind: OperationKind::Move,
                from: root.join("a.txt"),
                to: root.join("Docs/a.txt"),
                size_bytes: 1,
            }],
        );

        let result = undo(&record).unwrap();
        assert_eq!(result.restored, 0);
        assert_eq!(result.conflicts.len(), 1);
    }

    fn moved(id: &str, from: std::path::PathBuf, to: std::path::PathBuf) -> AppliedOperation {
        AppliedOperation {
            id: id.into(),
            kind: OperationKind::Move,
            from,
            to,
            size_bytes: 1,
        }
    }

    #[test]
    fn removes_folders_the_run_left_empty_but_never_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("2026/01")).unwrap();
        fs::write(root.join("2026/01/a.txt"), b"a").unwrap();

        let record = make_record(
            root,
            vec![moved("1", root.join("a.txt"), root.join("2026/01/a.txt"))],
        );
        undo(&record).unwrap();

        assert!(root.join("a.txt").exists());
        assert!(!root.join("2026").exists());
        assert!(root.exists());
    }

    #[test]
    fn keeps_folders_that_still_hold_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Docs")).unwrap();
        fs::write(root.join("Docs/a.txt"), b"a").unwrap();
        fs::write(root.join("Docs/keep.txt"), b"keep").unwrap();

        let record = make_record(
            root,
            vec![moved("1", root.join("a.txt"), root.join("Docs/a.txt"))],
        );
        undo(&record).unwrap();

        assert!(root.join("Docs/keep.txt").exists());
    }

    /// Regression: a run whose undo hit a conflict used to be marked undone
    /// anyway, so the files that couldn't be restored could never be retried.
    #[test]
    fn a_partial_undo_can_be_retried_once_the_conflict_is_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Docs")).unwrap();
        fs::write(root.join("Docs/a.txt"), b"a").unwrap();
        fs::write(root.join("Docs/b.txt"), b"b").unwrap();
        fs::write(root.join("b.txt"), b"squatter").unwrap();

        let mut record = make_record(
            root,
            vec![
                moved("a", root.join("a.txt"), root.join("Docs/a.txt")),
                moved("b", root.join("b.txt"), root.join("Docs/b.txt")),
            ],
        );

        let first = undo_and_record(&mut record).unwrap();
        assert_eq!(first.restored, 1);
        assert_eq!(first.conflicts.len(), 1);
        assert!(!record.undone);
        assert_eq!(record.restored_ids, vec!["a".to_string()]);

        fs::remove_file(root.join("b.txt")).unwrap();
        let second = undo_and_record(&mut record).unwrap();
        assert_eq!(second.restored, 1);
        assert!(second.conflicts.is_empty());
        assert!(record.undone);
        assert_eq!(fs::read(root.join("b.txt")).unwrap(), b"b");
    }
}
