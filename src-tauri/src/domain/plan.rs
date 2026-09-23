use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// Move a file to a new organized location.
    Move,
    /// Move a file into the `.sortty-trash` staging folder (used for duplicates and
    /// "cleanup" mode) instead of deleting it for real.
    MoveToTrash,
    /// Move a file into the `.sortty-archive` staging folder (used for stale-file cleanup).
    Archive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanMode {
    SortByType,
    SortByDate,
    Dedup,
    Cleanup,
    /// A manual delete of specific files, initiated from the folder browser
    /// rather than a scan — see `commands::browse::delete_files`.
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub kind: OperationKind,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub reason: String,
    pub size_bytes: u64,
    pub selected: bool,
}

impl Operation {
    pub fn new(
        kind: OperationKind,
        source: PathBuf,
        destination: PathBuf,
        reason: impl Into<String>,
        size_bytes: u64,
    ) -> Self {
        Operation {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            source,
            destination,
            reason: reason.into(),
            size_bytes,
            selected: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanSummary {
    pub total_files: usize,
    pub total_bytes: u64,
    pub per_destination_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub root: PathBuf,
    pub mode: PlanMode,
    pub created_at: DateTime<Utc>,
    pub operations: Vec<Operation>,
    pub summary: PlanSummary,
}

impl Plan {
    pub fn new(root: PathBuf, mode: PlanMode, operations: Vec<Operation>) -> Self {
        let mut summary = PlanSummary::default();
        summary.total_files = operations.len();
        for op in &operations {
            summary.total_bytes += op.size_bytes;
            let key = op
                .destination
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            *summary.per_destination_counts.entry(key).or_insert(0) += 1;
        }
        Plan {
            id: uuid::Uuid::new_v4().to_string(),
            root,
            mode,
            created_at: Utc::now(),
            operations,
            summary,
        }
    }
}
