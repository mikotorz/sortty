export type OperationKind = "move" | "move_to_trash" | "archive";
export type PlanMode =
  "sort_by_type" | "sort_by_date" | "dedup" | "cleanup" | "delete";
export type DateSource = "modified" | "created";
export type DateGranularity = "year" | "year_month";
export type KeepStrategy = "oldest_modified" | "shortest_path";
export type StaleAction = "archive" | "flag_only";

export interface FileEntry {
  path: string;
  file_name: string;
  extension: string | null;
  size_bytes: number;
  modified: string;
  created: string | null;
}

export interface ScanOptions {
  include_subfolders: boolean;
  exclude_folders: string[];
}

/** Progress while building a plan. `checking`/`comparing` are Find
 * Duplicates' two hashing passes (first 64 KiB, then full contents). */
export type PlanProgress =
  | { phase: "scanning"; count: number }
  | { phase: "checking"; done: number; total: number }
  | { phase: "comparing"; done: number; total: number };

export interface Operation {
  id: string;
  kind: OperationKind;
  source: string;
  destination: string;
  reason: string;
  size_bytes: number;
  selected: boolean;
}

export interface PlanSummary {
  total_files: number;
  total_bytes: number;
  per_destination_counts: Record<string, number>;
}

export interface Plan {
  id: string;
  root: string;
  mode: PlanMode;
  created_at: string;
  operations: Operation[];
  summary: PlanSummary;
}

export type PlanRequest =
  | { mode: "sort_by_type" }
  | {
      mode: "sort_by_date";
      date_source: DateSource;
      granularity: DateGranularity;
    }
  | { mode: "dedup"; min_size_bytes: number; keep_strategy: KeepStrategy }
  | {
      mode: "cleanup";
      stale_days: number;
      date_source: DateSource;
      action: StaleAction;
    };

export interface AppliedOperation {
  id: string;
  kind: OperationKind;
  from: string;
  to: string;
  size_bytes: number;
}

export interface FailedOperation {
  operation: Operation;
  error: string;
}

export interface RunRecord {
  run_id: string;
  root: string;
  plan_mode: PlanMode;
  started_at: string;
  finished_at: string;
  applied_operations: AppliedOperation[];
  failed_operations: FailedOperation[];
  undone: boolean;
  cancelled: boolean;
  restored_ids: string[];
}

export interface RunSummary {
  run_id: string;
  root: string;
  plan_mode: PlanMode;
  started_at: string;
  applied_count: number;
  failed_count: number;
  undone: boolean;
  cancelled: boolean;
  partially_undone: boolean;
}

export interface UndoResult {
  restored: number;
  conflicts: AppliedOperation[];
}

export interface CategoryDef {
  extensions: string[];
}

export interface CategoryRules {
  categories: Record<string, CategoryDef>;
  other_folder_name: string;
}

export type StagingKind = "trash" | "archive";

export interface EmptyResult {
  deleted_files: number;
  freed_bytes: number;
}

export interface AppSettings {
  general: { default_root: string | null; last_used_mode: PlanMode };
  sort_by_date: { granularity: DateGranularity; date_source: DateSource };
  cleanup: { stale_days: number; date_source: DateSource; action: StaleAction };
  dedup: { keep_strategy: KeepStrategy; min_size_bytes: number };
  trash: { staging_folder_name: string; archive_folder_name: string };
}
