use serde::Deserialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::config::settings;
use crate::domain::plan::Plan;
use crate::engine::cleanup::{self, CleanupOptions, StaleAction};
use crate::engine::dedup::{self, DedupOptions, KeepStrategy};
use crate::engine::scanner::{self, ScanOptions};
use crate::engine::sort_by_date::{self, DateGranularity, DateSource, SortByDateOptions};
use crate::engine::sort_by_type;
use crate::error::AppError;

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

#[tauri::command]
pub async fn generate_plan(
    app: AppHandle,
    root: String,
    scan_options: Option<ScanOptions>,
    request: PlanRequest,
) -> Result<Plan, AppError> {
    let root_path = PathBuf::from(root);
    let scan_options = scan_options.unwrap_or_default();
    let entries = scanner::scan(&root_path, &scan_options)?;

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

    Ok(plan)
}
