import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  CategoryRules,
  EmptyResult,
  FileEntry,
  Plan,
  PlanProgress,
  PlanRequest,
  RunRecord,
  RunSummary,
  ScanOptions,
  StagingKind,
  UndoResult,
} from "./types";

/** Resolves to `null` if the user cancelled the scan before a plan existed. */
export function generatePlan(
  root: string,
  request: PlanRequest,
  scanOptions?: ScanOptions,
  onProgress?: (progress: PlanProgress) => void,
): Promise<Plan | null> {
  const channel = new Channel<PlanProgress>();
  if (onProgress) {
    channel.onmessage = onProgress;
  }
  return invoke("generate_plan", {
    root,
    request,
    scanOptions,
    onProgress: channel,
  });
}

/** Applies the plan the backend is holding for `planId` (the id of the plan
 * last returned by `generatePlan`) — the plan itself is never sent back. */
export function applyPlan(
  planId: string,
  selectedIds: string[],
  onProgress?: (completed: number, total: number) => void,
): Promise<RunRecord> {
  const channel = new Channel<{ completed: number; total: number }>();
  if (onProgress) {
    channel.onmessage = (message) =>
      onProgress(message.completed, message.total);
  }
  return invoke("apply_plan", { planId, selectedIds, onProgress: channel });
}

export function cancelCurrentOperation(): Promise<void> {
  return invoke("cancel_current_operation");
}

export function listRuns(limit?: number): Promise<RunSummary[]> {
  return invoke("list_runs", { limit });
}

export function getRun(runId: string): Promise<RunRecord> {
  return invoke("get_run", { runId });
}

export function undoRun(runId: string): Promise<UndoResult> {
  return invoke("undo_run", { runId });
}

export function undoLastRun(): Promise<UndoResult> {
  return invoke("undo_last_run");
}

export function getCategoryRules(): Promise<CategoryRules> {
  return invoke("get_category_rules");
}

export function saveCategoryRules(rules: CategoryRules): Promise<void> {
  return invoke("save_category_rules", { rules });
}

export function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export function saveSettings(settingsValue: AppSettings): Promise<void> {
  return invoke("save_settings", { settingsValue });
}

export function resetSettings(): Promise<AppSettings> {
  return invoke("reset_settings");
}

export function resetCategoryRules(): Promise<CategoryRules> {
  return invoke("reset_category_rules");
}

export function openConfigFolder(): Promise<void> {
  return invoke("open_config_folder");
}

export function readFilePreview(path: string): Promise<string> {
  return invoke("read_file_preview", { path });
}

export function browseFolder(path: string): Promise<FileEntry[]> {
  return invoke("browse_folder", { path });
}

export function deleteFiles(root: string, paths: string[]): Promise<RunRecord> {
  return invoke("delete_files", { root, paths });
}

export function previewStagingFolder(
  root: string,
  kind: StagingKind,
): Promise<EmptyResult> {
  return invoke("preview_staging_folder", { root, kind });
}

export function emptyStagingFolder(
  root: string,
  kind: StagingKind,
): Promise<EmptyResult> {
  return invoke("empty_staging_folder", { root, kind });
}
