use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::commands::cancel::CancelFlag;
use crate::config::settings;
use crate::domain::plan::Plan;
use crate::engine::cleanup::{self, CleanupOptions, StaleAction};
use crate::engine::dedup::{self, DedupOptions, KeepStrategy};
use crate::engine::scanner::{self, ScanOptions};
use crate::engine::sort_by_date::{self, DateGranularity, DateSource, SortByDateOptions};
use crate::engine::sort_by_type;
use crate::error::AppError;

/// A running "N scanned so far" count — there's no total, since `scan`'s
/// underlying walk is a lazy single-pass iterator that doesn't know the file
/// count ahead of time.
#[derive(Clone, Serialize)]
pub struct ScanProgress {
    pub count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PlanRequest {
    SortByType,
    SortByDate {
        date_source: DateSource,
        granularity: DateGranularity,
    },
    Dedup {
        min_size_bytes: u64,
        keep_strategy: KeepStrategy,
    },
    Cleanup {
        stale_days: u32,
        date_source: DateSource,
        action: StaleAction,
    },
}

/// Scans `root` and builds a plan for `request`, reading category rules and
/// staging folder names from `config_dir`. Returns `Ok(None)` if
/// `should_cancel` fired mid-scan. The staging folders are always excluded
/// from the scan, whatever `scan_options.exclude` says (ADR 0017).
pub fn generate_plan_at(
    root: &Path,
    scan_options: ScanOptions,
    request: PlanRequest,
    config_dir: &Path,
    on_progress: impl FnMut(usize),
    should_cancel: impl Fn() -> bool,
) -> Result<Option<Plan>, AppError> {
    let app_settings = settings::load_settings(config_dir)?;
    let scan_options = scan_options.excluding(app_settings.trash.staging_folder_names());

    let outcome = scanner::scan_with_progress(root, &scan_options, on_progress, should_cancel)?;
    if outcome.cancelled {
        return Ok(None);
    }
    let entries = outcome.entries;

    let plan = match request {
        PlanRequest::SortByType => {
            let rules = settings::load_category_rules(config_dir)?;
            sort_by_type::build_plan(root, &entries, &rules)
        }
        PlanRequest::SortByDate {
            date_source,
            granularity,
        } => sort_by_date::build_plan(
            root,
            &entries,
            &SortByDateOptions {
                date_source,
                granularity,
            },
        ),
        PlanRequest::Dedup {
            min_size_bytes,
            keep_strategy,
        } => dedup::build_plan(
            root,
            &entries,
            &DedupOptions {
                min_size_bytes,
                keep_strategy,
                trash_folder_name: app_settings.trash.staging_folder_name,
            },
        )?,
        PlanRequest::Cleanup {
            stale_days,
            date_source,
            action,
        } => cleanup::build_plan(
            root,
            &entries,
            &CleanupOptions {
                stale_days,
                date_source,
                action,
                archive_folder_name: app_settings.trash.archive_folder_name,
            },
            chrono::Utc::now(),
        ),
    };

    Ok(Some(plan))
}

/// Returns `Ok(None)` if the user cancelled the scan before a plan could be
/// generated — there's nothing partial to salvage from an interrupted scan,
/// since a plan needs the complete entry list to be built from.
#[tauri::command]
pub async fn generate_plan(
    app: AppHandle,
    cancel_flag: State<'_, CancelFlag>,
    root: String,
    scan_options: Option<ScanOptions>,
    request: PlanRequest,
    on_progress: Channel<ScanProgress>,
) -> Result<Option<Plan>, AppError> {
    cancel_flag.reset();
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;

    let mut last_sent = 0usize;
    generate_plan_at(
        &PathBuf::from(root),
        scan_options.unwrap_or_default(),
        request,
        &config_dir,
        |count| {
            if count - last_sent >= 50 {
                on_progress.send(ScanProgress { count }).ok();
                last_sent = count;
            }
        },
        || cancel_flag.is_cancelled(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn recursive_with_empty_exclude() -> ScanOptions {
        ScanOptions {
            include_subfolders: true,
            exclude: Vec::new(),
            exclude_folders: Vec::new(),
        }
    }

    /// Regression: the frontend always sent `exclude: []`, which replaced
    /// the scanner's default exclusion, so a recursive Sort by Type moved
    /// staged duplicates back out of `.sortty-trash` into `Images/` etc.
    #[test]
    fn recursive_plan_never_touches_staging_folders_even_with_empty_exclude() {
        let root = tempfile::tempdir().unwrap();
        let config = tempfile::tempdir().unwrap();
        fs::write(root.path().join("loose.png"), b"a").unwrap();
        for staging in [".sortty-trash", ".sortty-archive"] {
            fs::create_dir(root.path().join(staging)).unwrap();
            fs::write(root.path().join(staging).join("staged.png"), b"b").unwrap();
        }

        let plan = generate_plan_at(
            root.path(),
            recursive_with_empty_exclude(),
            PlanRequest::SortByType,
            config.path(),
            |_| {},
            || false,
        )
        .unwrap()
        .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].source, root.path().join("loose.png"));
    }

    #[test]
    fn recursive_plan_skips_a_renamed_trash_folder_and_the_old_default() {
        let root = tempfile::tempdir().unwrap();
        let config = tempfile::tempdir().unwrap();
        let mut app_settings = settings::AppSettings::default();
        app_settings.trash.staging_folder_name = "MyTrash".to_string();
        settings::save_settings(config.path(), &app_settings).unwrap();

        fs::write(root.path().join("loose.png"), b"a").unwrap();
        for staging in ["MyTrash", ".sortty-trash"] {
            fs::create_dir(root.path().join(staging)).unwrap();
            fs::write(root.path().join(staging).join("staged.png"), b"b").unwrap();
        }

        let plan = generate_plan_at(
            root.path(),
            recursive_with_empty_exclude(),
            PlanRequest::SortByType,
            config.path(),
            |_| {},
            || false,
        )
        .unwrap()
        .unwrap();

        assert_eq!(plan.operations.len(), 1);
    }
}
