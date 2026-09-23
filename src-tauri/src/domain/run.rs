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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UndoResult {
    pub restored: usize,
    pub conflicts: Vec<AppliedOperation>,
}
