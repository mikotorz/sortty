import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  CategoryRules,
  Plan,
  PlanRequest,
  RunRecord,
  RunSummary,
  ScanOptions,
  ScanResult,
  UndoResult,
} from "./types";

export function scanFolder(root: string, options?: ScanOptions): Promise<ScanResult> {
  return invoke("scan_folder", { root, options });
}

export function generatePlan(
  root: string,
  request: PlanRequest,
  scanOptions?: ScanOptions,
): Promise<Plan> {
  return invoke("generate_plan", { root, request, scanOptions });
}

export function applyPlan(plan: Plan, selectedIds: string[]): Promise<RunRecord> {
  return invoke("apply_plan", { plan, selectedIds });
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

export function openConfigFolder(): Promise<void> {
  return invoke("open_config_folder");
}
