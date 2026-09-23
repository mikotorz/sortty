use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::domain::entry::FileEntry;
use crate::domain::plan::{Operation, OperationKind, Plan, PlanMode};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DateSource {
    Modified,
    Created,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DateGranularity {
    Year,
    YearMonth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortByDateOptions {
    pub date_source: DateSource,
    pub granularity: DateGranularity,
}

pub fn build_plan(root: &Path, entries: &[FileEntry], options: &SortByDateOptions) -> Plan {
    let mut operations = Vec::new();

    for entry in entries {
        let date = match options.date_source {
            DateSource::Modified => entry.modified,
            DateSource::Created => entry.created.unwrap_or(entry.modified),
        };

        let dest_dir = match options.granularity {
            DateGranularity::Year => root.join(format!("{:04}", date.year())),
            DateGranularity::YearMonth => root
                .join(format!("{:04}", date.year()))
                .join(format!("{:02}", date.month())),
        };
        let destination = dest_dir.join(&entry.file_name);

        if entry.path == destination {
            continue;
        }

        let reason = format!("Dated {}", date.format("%Y-%m-%d"));
        operations.push(Operation::new(
            OperationKind::Move,
            entry.path.clone(),
            destination,
            reason,
            entry.size_bytes,
        ));
    }

    Plan::new(root.to_path_buf(), PlanMode::SortByDate, operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(name: &str, modified: chrono::DateTime<chrono::Utc>) -> FileEntry {
        FileEntry {
            path: std::path::PathBuf::from(format!("/root/{name}")),
            file_name: name.to_string(),
            extension: None,
            size_bytes: 1,
            modified,
            created: None,
        }
    }

    #[test]
    fn buckets_by_year_month() {
        let date = chrono::Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        let entries = vec![entry("a.txt", date)];
        let options = SortByDateOptions {
            date_source: DateSource::Modified,
            granularity: DateGranularity::YearMonth,
        };
        let plan = build_plan(Path::new("/root"), &entries, &options);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/2026/01/a.txt")
        );
    }

    #[test]
    fn buckets_by_year_only() {
        let date = chrono::Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();
        let entries = vec![entry("a.txt", date)];
        let options = SortByDateOptions {
            date_source: DateSource::Modified,
            granularity: DateGranularity::Year,
        };
        let plan = build_plan(Path::new("/root"), &entries, &options);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/2025/a.txt")
        );
    }
}
