use crate::domain::run::{RunRecord, UndoResult};
use crate::error::AppError;

/// Reverses a run's applied operations (`to -> from`), most recent first.
/// A destination that's occupied again (something new landed there since the
/// run) is skipped and reported as a conflict rather than aborting the undo.
pub fn undo(record: &RunRecord) -> Result<UndoResult, AppError> {
    let mut restored = 0;
    let mut conflicts = Vec::new();

    for applied in record.applied_operations.iter().rev() {
        if !applied.to.exists() {
            // Already moved/removed by something else; nothing to restore.
            conflicts.push(applied.clone());
            continue;
        }
        if applied.from.exists() {
            // Something already occupies the original path.
            conflicts.push(applied.clone());
            continue;
        }

        if let Some(parent) = applied.from.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent.to_path_buf(), e))?;
        }

        match std::fs::rename(&applied.to, &applied.from) {
            Ok(()) => restored += 1,
            Err(_) => {
                // Fall back to copy + remove for cross-volume moves.
                match std::fs::copy(&applied.to, &applied.from)
                    .and_then(|_| std::fs::remove_file(&applied.to))
                {
                    Ok(()) => restored += 1,
                    Err(_) => conflicts.push(applied.clone()),
                }
            }
        }
    }

    Ok(UndoResult {
        restored,
        conflicts,
    })
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
}
