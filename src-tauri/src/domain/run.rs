use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::plan::{Operation, OperationKind, PlanMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedOperation {
    pub id: String,
    pub kind: OperationKind,
    pub from: PathBuf,
    pub to: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedOperation {
    pub operation: Operation,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: String,
    pub root: PathBuf,
    pub plan_mode: PlanMode,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub applied_operations: Vec<AppliedOperation>,
    pub failed_operations: Vec<FailedOperation>,
    pub undone: bool,
    /// True if the user cancelled this run partway through — `applied_operations`
    /// then holds only what was actually moved before the cancellation, not the
    /// full plan. Defaults to `false` so runs persisted before this field existed
    /// still deserialize.
    #[serde(default)]
    pub cancelled: bool,
    /// Ids of the `applied_operations` an undo has already put back. An undo
    /// that hits conflicts restores what it can and records it here, so a
    /// retry only attempts the rest; `undone` becomes true only once every
    /// operation is restored.
    #[serde(default)]
    pub restored_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    pub root: PathBuf,
    pub plan_mode: PlanMode,
    pub started_at: DateTime<Utc>,
    pub applied_count: usize,
    pub failed_count: usize,
    pub undone: bool,
    #[serde(default)]
    pub cancelled: bool,
    /// Some, but not all, of the run's operations have been undone.
    #[serde(default)]
    pub partially_undone: bool,
}

impl From<&RunRecord> for RunSummary {
    fn from(r: &RunRecord) -> Self {
        RunSummary {
            run_id: r.run_id.clone(),
            root: r.root.clone(),
            plan_mode: r.plan_mode,
            started_at: r.started_at,
            applied_count: r.applied_operations.len(),
            failed_count: r.failed_operations.len(),
            undone: r.undone,
            cancelled: r.cancelled,
            partially_undone: !r.undone && !r.restored_ids.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UndoResult {
    pub restored: usize,
    pub conflicts: Vec<AppliedOperation>,
    /// Ids restored by this undo attempt, for the caller to persist into
    /// `RunRecord::restored_ids`. Not sent to the frontend.
    #[serde(skip)]
    pub restored_ids: Vec<String>,
}
