use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::domain::entry::FileEntry;
use crate::domain::plan::{Operation, OperationKind, Plan, PlanMode};
use crate::engine::sort_by_date::DateSource;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StaleAction {
    /// Pre-select the moves into `.sortty-archive` for the user.
    Archive,
    /// Show the stale files in the preview but leave them unchecked, so the
    /// user opts in per-file rather than archiving everything at once.
    FlagOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupOptions {
    pub stale_days: u32,
    pub date_source: DateSource,
    pub action: StaleAction,
    pub archive_folder_name: String,
}

fn archive_destination(root: &Path, source: &Path, archive_folder_name: &str) -> PathBuf {
    let relative = source.strip_prefix(root).unwrap_or(source);
    root.join(archive_folder_name).join(relative)
}

pub fn build_plan(
    root: &Path,
    entries: &[FileEntry],
    options: &CleanupOptions,
    now: DateTime<Utc>,
) -> Plan {
    let threshold = now - chrono::Duration::days(options.stale_days as i64);
    let mut operations = Vec::new();

    for entry in entries {
        let relevant_date = match options.date_source {
            DateSource::Modified => entry.modified,
            DateSource::Created => entry.created.unwrap_or(entry.modified),
        };

        if relevant_date > threshold {
            continue;
        }

        let age_days = (now - relevant_date).num_days();
        let mut op = Operation::new(
            OperationKind::Archive,
            entry.path.clone(),
            archive_destination(root, &entry.path, &options.archive_folder_name),
            format!("Stale, last touched {age_days} days ago"),
            entry.size_bytes,
        );
        if options.action == StaleAction::FlagOnly {
            op.selected = false;
        }
        operations.push(op);
    }

    Plan::new(root.to_path_buf(), PlanMode::Cleanup, operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry_at(days_old: i64, now: DateTime<Utc>) -> FileEntry {
        FileEntry {
            path: PathBuf::from(format!("/root/f{days_old}.txt")),
            file_name: format!("f{days_old}.txt"),
            extension: Some("txt".into()),
            size_bytes: 1,
            modified: now - chrono::Duration::days(days_old),
            created: None,
        }
    }

    #[test]
    fn boundary_is_inclusive_at_exactly_stale_days() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let entries = vec![entry_at(179, now), entry_at(180, now), entry_at(181, now)];
        let options = CleanupOptions {
            stale_days: 180,
            date_source: DateSource::Modified,
            action: StaleAction::Archive,
            archive_folder_name: ".sortty-archive".to_string(),
        };
        let plan = build_plan(Path::new("/root"), &entries, &options, now);
        assert_eq!(plan.operations.len(), 2); // 180 and 181 days old, not 179
    }

    #[test]
    fn flag_only_leaves_operations_unselected() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let entries = vec![entry_at(200, now)];
        let options = CleanupOptions {
            stale_days: 180,
            date_source: DateSource::Modified,
            action: StaleAction::FlagOnly,
            archive_folder_name: ".sortty-archive".to_string(),
        };
        let plan = build_plan(Path::new("/root"), &entries, &options, now);
        assert!(!plan.operations[0].selected);
    }
}
