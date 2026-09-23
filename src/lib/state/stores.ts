import { writable } from "svelte/store";
import type { AppSettings, CategoryRules, Plan, RunRecord, RunSummary } from "../api/types";

export type View = "scan" | "preview" | "settings" | "history";

export const currentView = writable<View>("scan");
export const selectedRoot = writable<string | null>(null);
export const currentPlan = writable<Plan | null>(null);
export const selectedOperationIds = writable<Record<string, boolean>>({});
export const settings = writable<AppSettings | null>(null);
export const categoryRules = writable<CategoryRules | null>(null);
export const historyRuns = writable<RunSummary[]>([]);
export const busy = writable(false);
export const statusMessage = writable<string | null>(null);
export const lastRunRecord = writable<RunRecord | null>(null);
