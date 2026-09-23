use chrono::{Datelike, Local, TimeZone};
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
    build_plan_in(root, entries, options, &Local)
}

/// `build_plan`, bucketing dates in `tz` rather than the machine's local
/// time zone. Buckets must follow the user's calendar, not UTC: a file saved
/// at 00:30 on 1 January in UTC+1 belongs in the new year's folder.
pub fn build_plan_in<Tz: TimeZone>(
    root: &Path,
    entries: &[FileEntry],
    options: &SortByDateOptions,
    tz: &Tz,
) -> Plan
where
    Tz::Offset: std::fmt::Display,
{
    let mut operations = Vec::new();

    for entry in entries {
        let date = match options.date_source {
            DateSource::Modified => entry.modified,
            DateSource::Created => entry.created.unwrap_or(entry.modified),
        }
        .with_timezone(tz);

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
        let plan = build_plan_in(Path::new("/root"), &entries, &options, &chrono::Utc);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/2026/01/a.txt")
        );
    }

    /// Regression: dates were bucketed in UTC, so a New Year's Eve file in
    /// a time zone ahead of UTC landed in the previous year's folder.
    #[test]
    fn buckets_in_the_given_time_zone_not_utc() {
        // 2026-01-01 00:30 in UTC+1 is still 2025-12-31 in UTC.
        let date = chrono::Utc
            .with_ymd_and_hms(2025, 12, 31, 23, 30, 0)
            .unwrap();
        let entries = vec![entry("a.txt", date)];
        let options = SortByDateOptions {
            date_source: DateSource::Modified,
            granularity: DateGranularity::YearMonth,
        };
        let utc_plus_one = chrono::FixedOffset::east_opt(3600).unwrap();
        let plan = build_plan_in(Path::new("/root"), &entries, &options, &utc_plus_one);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/2026/01/a.txt")
        );
        assert_eq!(plan.operations[0].reason, "Dated 2026-01-01");
    }

    #[test]
    fn buckets_by_year_only() {
        let date = chrono::Utc
            .with_ymd_and_hms(2025, 12, 31, 23, 59, 59)
            .unwrap();
        let entries = vec![entry("a.txt", date)];
        let options = SortByDateOptions {
            date_source: DateSource::Modified,
            granularity: DateGranularity::Year,
        };
        let plan = build_plan_in(Path::new("/root"), &entries, &options, &chrono::Utc);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/2025/a.txt")
        );
    }
}
