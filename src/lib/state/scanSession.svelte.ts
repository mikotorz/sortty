import type { Plan, PlanRequest, RunRecord, ScanOptions } from "../api/types";

/**
 * Sort & Clean's in-progress scan/plan state, lifted out of the page component
 * so it survives navigating to History/Settings and back — routes unmount
 * their page component, which would otherwise reset any local $state.
 */
class ScanSession {
  request = $state<PlanRequest>({ mode: "sort_by_type" });
  scanOptions = $state<ScanOptions>({ include_subfolders: false, exclude: [] });
  plan = $state<Plan | null>(null);
  selected = $state<Record<string, boolean>>({});
  scanning = $state(false);
  applying = $state(false);
  lastRun = $state<RunRecord | null>(null);
  showFailures = $state(false);
}

export const scanSession = new ScanSession();
