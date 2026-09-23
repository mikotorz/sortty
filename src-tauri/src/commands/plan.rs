use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    let root_path = PathBuf::from(root);
    let scan_options = scan_options.unwrap_or_default();

    let mut last_sent = 0usize;
    let outcome = scanner::scan_with_progress(
        &root_path,
        &scan_options,
        |count| {
            if count - last_sent >= 50 {
                on_progress.send(ScanProgress { count }).ok();
                last_sent = count;
            }
        },
        || cancel_flag.is_cancelled(),
    )?;
    if outcome.cancelled {
        return Ok(None);
    }
    let entries = outcome.entries;

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;

    let plan = match request {
        PlanRequest::SortByType => {
            let rules = settings::load_category_rules(&config_dir)?;
            sort_by_type::build_plan(&root_path, &entries, &rules)
        }
        PlanRequest::SortByDate {
            date_source,
            granularity,
        } => sort_by_date::build_plan(
            &root_path,
            &entries,
            &SortByDateOptions {
                date_source,
                granularity,
            },
        ),
        PlanRequest::Dedup {
            min_size_bytes,
            keep_strategy,
        } => {
            let app_settings = settings::load_settings(&config_dir)?;
            dedup::build_plan(
                &root_path,
                &entries,
                &DedupOptions {
                    min_size_bytes,
                    keep_strategy,
                    trash_folder_name: app_settings.trash.staging_folder_name,
                },
            )?
        }
        PlanRequest::Cleanup {
            stale_days,
            date_source,
            action,
        } => {
            let app_settings = settings::load_settings(&config_dir)?;
            cleanup::build_plan(
                &root_path,
                &entries,
                &CleanupOptions {
                    stale_days,
                    date_source,
                    action,
                    archive_folder_name: app_settings.trash.archive_folder_name,
                },
                chrono::Utc::now(),
            )
        }
    };

    Ok(Some(plan))
}
