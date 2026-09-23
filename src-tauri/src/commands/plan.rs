use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::commands::blocking;
use crate::commands::cancel::CancelFlag;
use crate::config::settings;
use crate::domain::plan::Plan;
use crate::engine::cleanup::{self, CleanupOptions, StaleAction};
use crate::engine::dedup::{self, DedupOptions, HashStage, KeepStrategy};
use crate::engine::scanner::{self, ScanOptions};
use crate::engine::sort_by_date::{self, DateGranularity, DateSource, SortByDateOptions};
use crate::engine::sort_by_type;
use crate::error::AppError;

/// The plan the user is currently previewing, kept in the backend so
/// `apply_plan` can look it up by id instead of accepting whatever `Plan`
/// the webview sends back (ADR 0016). Holds only the latest plan: a new scan
/// replaces it, and applying it takes it out.
#[derive(Clone, Default)]
pub struct PlanStore(Arc<Mutex<Option<Plan>>>);

impl PlanStore {
    pub fn put(&self, plan: Plan) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(plan);
    }

    /// Removes and returns the stored plan if its id is `plan_id`.
    pub fn take(&self, plan_id: &str) -> Result<Plan, AppError> {
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_ref() {
            Some(plan) if plan.id == plan_id => Ok(slot.take().expect("checked above")),
            _ => Err(AppError::InvalidPlan(
                "this preview is out of date — scan again, then apply".to_string(),
            )),
        }
    }
}

/// Progress while building a plan, tagged by phase for the frontend:
/// `{ phase: "scanning", count }` while walking the folder (no total — the
/// walk is a lazy single-pass iterator that doesn't know the file count
/// ahead of time), then, for Find Duplicates only, `checking` and
/// `comparing` with a real `done`/`total` for the two hashing passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum PlanProgress {
    Scanning { count: usize },
    Checking { done: usize, total: usize },
    Comparing { done: usize, total: usize },
}

impl PlanProgress {
    /// Whether this report is worth sending over IPC: every 50th scanned
    /// entry, every 20th hashed file, and the last one of each hashing pass.
    fn worth_sending(&self) -> bool {
        match *self {
            PlanProgress::Scanning { count } => count % 50 == 0,
            PlanProgress::Checking { done, total } | PlanProgress::Comparing { done, total } => {
                done % 20 == 0 || done == total
            }
        }
    }
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
    on_progress: &(dyn Fn(PlanProgress) + Sync),
    should_cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Option<Plan>, AppError> {
    let app_settings = settings::load_settings(config_dir)?;
    let scan_options = scan_options.excluding(app_settings.trash.staging_folder_names());

    let outcome = scanner::scan_with_progress(
        root,
        &scan_options,
        |count| on_progress(PlanProgress::Scanning { count }),
        should_cancel,
    )?;
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
        } => {
            let plan = dedup::build_plan_with_progress(
                root,
                &entries,
                &DedupOptions {
                    min_size_bytes,
                    keep_strategy,
                    trash_folder_name: app_settings.trash.staging_folder_name,
                },
                &|stage, done, total| {
                    on_progress(match stage {
                        HashStage::Checking => PlanProgress::Checking { done, total },
                        HashStage::Comparing => PlanProgress::Comparing { done, total },
                    })
                },
                should_cancel,
            );
            match plan {
                Some(plan) => plan,
                None => return Ok(None),
            }
        }
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
    plan_store: State<'_, PlanStore>,
    root: String,
    scan_options: Option<ScanOptions>,
    request: PlanRequest,
    on_progress: Channel<PlanProgress>,
) -> Result<Option<Plan>, AppError> {
    cancel_flag.reset();
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;

    let cancel_flag = cancel_flag.inner().clone();
    let plan_store = plan_store.inner().clone();
    blocking(move || {
        let plan = generate_plan_at(
            &PathBuf::from(root),
            scan_options.unwrap_or_default(),
            request,
            &config_dir,
            &|progress| {
                if progress.worth_sending() {
                    on_progress.send(progress).ok();
                }
            },
            &|| cancel_flag.is_cancelled(),
        )?;
        if let Some(plan) = &plan {
            plan_store.put(plan.clone());
        }
        Ok(plan)
    })
    .await
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
            &|_| {},
            &|| false,
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
            &|_| {},
            &|| false,
        )
        .unwrap()
        .unwrap();

        assert_eq!(plan.operations.len(), 1);
    }
}
