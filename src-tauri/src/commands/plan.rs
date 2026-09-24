use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::commands::blocking;
use crate::commands::cancel::CancelFlag;
use crate::config::settings;
use crate::domain::plan::{duplicate_reason, staged_destination, Operation, OperationKind, Plan};
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
            _ => Err(stale_plan()),
        }
    }

    /// Edits the stored plan in place if its id is `plan_id`, returning the
    /// edited plan for the preview (ADR 0021). If `edit` fails, the stored
    /// plan is left exactly as it was.
    pub fn update(
        &self,
        plan_id: &str,
        edit: impl FnOnce(&mut Plan) -> Result<(), AppError>,
    ) -> Result<Plan, AppError> {
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_mut() {
            Some(plan) if plan.id == plan_id => {
                let mut edited = plan.clone();
                edit(&mut edited)?;
                *plan = edited.clone();
                Ok(edited)
            }
            _ => Err(stale_plan()),
        }
    }
}

fn stale_plan() -> AppError {
    AppError::InvalidPlan("this preview is out of date — scan again, then apply".to_string())
}

/// Find Duplicates: keep the copy that `operation_id` would have trashed,
/// and trash the set's current keeper instead. The webview names an
/// operation by id, never a path, so it still can't make the backend move a
/// file the scan didn't find (ADR 0016, ADR 0021).
pub fn choose_keeper_at(
    plans: &PlanStore,
    plan_id: &str,
    operation_id: &str,
) -> Result<Plan, AppError> {
    plans.update(plan_id, |plan| {
        let not_a_copy =
            || AppError::InvalidPlan("that file isn't a duplicate in this preview".to_string());
        let index = plan
            .operations
            .iter()
            .position(|op| op.id == operation_id)
            .ok_or_else(not_a_copy)?;
        let set_id = plan.operations[index]
            .duplicate_set
            .clone()
            .ok_or_else(not_a_copy)?;
        let root = plan.root.clone();
        let set = plan
            .duplicate_sets
            .iter_mut()
            .find(|s| s.id == set_id)
            .ok_or_else(not_a_copy)?;

        let promoted = plan.operations.remove(index);
        let old_keeper = std::mem::replace(&mut set.keeper, promoted.source.clone());
        let old_keeper_size = std::mem::replace(&mut set.keeper_size_bytes, promoted.size_bytes);
        let demoted = Operation::new(
            OperationKind::MoveToTrash,
            old_keeper.clone(),
            staged_destination(&root, &old_keeper, &set.trash_folder_name),
            duplicate_reason(&set.keeper),
            old_keeper_size,
        )
        .in_duplicate_set(&set_id);
        let reason = duplicate_reason(&set.keeper);

        for op in plan
            .operations
            .iter_mut()
            .filter(|op| op.duplicate_set.as_deref() == Some(&*set_id))
        {
            op.reason = reason.clone();
        }
        plan.operations.push(demoted);
        plan.operations.sort_by(|a, b| a.source.cmp(&b.source));
        plan.recompute_summary();
        Ok(())
    })
}

#[tauri::command]
pub fn choose_keeper(
    plan_store: State<'_, PlanStore>,
    plan_id: String,
    operation_id: String,
) -> Result<Plan, AppError> {
    choose_keeper_at(&plan_store, &plan_id, &operation_id)
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

    /// Three identical files: a.txt is kept, b.txt and c.txt are copies.
    fn dedup_plan_in_store(root: &Path, plans: &PlanStore) -> Plan {
        use crate::domain::plan::{DuplicateSet, PlanMode};
        let set_id = "set-1";
        let copy = |name: &str| {
            Operation::new(
                OperationKind::MoveToTrash,
                root.join(name),
                staged_destination(root, &root.join(name), ".sortty-trash"),
                duplicate_reason(&root.join("a.txt")),
                5,
            )
            .in_duplicate_set(set_id)
        };
        let mut plan = Plan::new(
            root.to_path_buf(),
            PlanMode::Dedup,
            vec![copy("b.txt"), copy("c.txt")],
        );
        plan.duplicate_sets = vec![DuplicateSet {
            id: set_id.to_string(),
            keeper: root.join("a.txt"),
            keeper_size_bytes: 5,
            trash_folder_name: ".sortty-trash".to_string(),
        }];
        plans.put(plan.clone());
        plan
    }

    fn sources(plan: &Plan) -> Vec<PathBuf> {
        plan.operations.iter().map(|op| op.source.clone()).collect()
    }

    #[test]
    fn choose_keeper_swaps_the_keeper_and_trashes_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let plans = PlanStore::default();
        let plan = dedup_plan_in_store(root, &plans);
        let c = plan.operations[1].id.clone();

        let edited = choose_keeper_at(&plans, &plan.id, &c).unwrap();

        assert_eq!(edited.id, plan.id);
        assert_eq!(edited.duplicate_sets[0].keeper, root.join("c.txt"));
        assert_eq!(
            sources(&edited),
            vec![root.join("a.txt"), root.join("b.txt")]
        );
        let a = &edited.operations[0];
        assert_eq!(a.kind, OperationKind::MoveToTrash);
        assert_eq!(a.destination, root.join(".sortty-trash").join("a.txt"));
        assert_eq!(a.duplicate_set.as_deref(), Some("set-1"));
        assert!(a.selected);
        let reason = duplicate_reason(&root.join("c.txt"));
        assert!(edited.operations.iter().all(|op| op.reason == reason));
        assert_eq!(edited.summary.total_files, 2);

        // The store now holds the edited plan, so Apply runs what's on screen.
        let stored = plans.take(&plan.id).unwrap();
        assert_eq!(sources(&stored), sources(&edited));
    }

    #[test]
    fn choosing_the_original_keeper_again_restores_the_plan() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let plans = PlanStore::default();
        let plan = dedup_plan_in_store(root, &plans);

        let edited = choose_keeper_at(&plans, &plan.id, &plan.operations[0].id).unwrap();
        let a = edited
            .operations
            .iter()
            .find(|op| op.source == root.join("a.txt"))
            .unwrap();
        let back = choose_keeper_at(&plans, &plan.id, &a.id).unwrap();

        assert_eq!(back.duplicate_sets[0].keeper, root.join("a.txt"));
        assert_eq!(sources(&back), sources(&plan));
    }

    #[test]
    fn choose_keeper_rejects_a_stale_plan_or_unknown_operation() {
        let dir = tempfile::tempdir().unwrap();
        let plans = PlanStore::default();
        let plan = dedup_plan_in_store(dir.path(), &plans);

        let stale = choose_keeper_at(&plans, "old-plan", &plan.operations[0].id);
        assert!(matches!(stale, Err(AppError::InvalidPlan(_))));
        let unknown = choose_keeper_at(&plans, &plan.id, "no-such-op");
        assert!(matches!(unknown, Err(AppError::InvalidPlan(_))));

        // A failed edit leaves the stored plan untouched.
        assert_eq!(sources(&plans.take(&plan.id).unwrap()), sources(&plan));
    }

    #[test]
    fn choose_keeper_rejects_an_operation_outside_any_duplicate_set() {
        use crate::domain::plan::PlanMode;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let plans = PlanStore::default();
        let op = Operation::new(
            OperationKind::Move,
            root.join("a.txt"),
            root.join("Docs/a.txt"),
            "sort",
            1,
        );
        let plan = Plan::new(root.to_path_buf(), PlanMode::SortByType, vec![op]);
        plans.put(plan.clone());

        let result = choose_keeper_at(&plans, &plan.id, &plan.operations[0].id);
        assert!(matches!(result, Err(AppError::InvalidPlan(_))));
    }
}
