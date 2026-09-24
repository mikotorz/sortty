use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// For Find Duplicates: the id of the [`DuplicateSet`] this copy belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_set: Option<String>,
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
            duplicate_set: None,
        }
    }

    pub fn in_duplicate_set(mut self, set_id: &str) -> Self {
        self.duplicate_set = Some(set_id.to_string());
        self
    }
}

/// A group of files with identical contents found by Find Duplicates. The
/// keeper stays where it is; every other copy is a `MoveToTrash` operation
/// tagged with this set's id. The user can swap the keeper (ADR 0021).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateSet {
    pub id: String,
    pub keeper: PathBuf,
    pub keeper_size_bytes: u64,
    /// The trash folder the copies go to, so a new copy (the old keeper,
    /// after a swap) is staged the same way as the rest.
    pub trash_folder_name: String,
}

/// The reason shown for a duplicate copy in the preview.
pub fn duplicate_reason(keeper: &Path) -> String {
    format!("Duplicate of {}", keeper.display())
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
    /// Find Duplicates only: each set's keeper and id. Empty for other modes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate_sets: Vec<DuplicateSet>,
}

/// The destination for a file being staged into a tool-owned folder inside
/// `root` (`.sortty-trash` for a duplicate, `.sortty-archive` for a stale
/// file), preserving its path relative to `root` underneath that folder.
/// Shared by dedup and cleanup so both stage files the same way.
pub fn staged_destination(root: &Path, source: &Path, staging_folder_name: &str) -> PathBuf {
    let relative = source.strip_prefix(root).unwrap_or(source);
    root.join(staging_folder_name).join(relative)
}

impl Plan {
    pub fn new(root: PathBuf, mode: PlanMode, operations: Vec<Operation>) -> Self {
        let mut plan = Plan {
            id: uuid::Uuid::new_v4().to_string(),
            root,
            mode,
            created_at: Utc::now(),
            operations,
            summary: PlanSummary::default(),
            duplicate_sets: Vec::new(),
        };
        plan.recompute_summary();
        plan
    }

    /// Rebuilds `summary` from `operations`, e.g. after a plan edit.
    pub fn recompute_summary(&mut self) {
        let mut summary = PlanSummary {
            total_files: self.operations.len(),
            ..Default::default()
        };
        for op in &self.operations {
            summary.total_bytes += op.size_bytes;
            let key = op
                .destination
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            *summary.per_destination_counts.entry(key).or_insert(0) += 1;
        }
        self.summary = summary;
    }
}
