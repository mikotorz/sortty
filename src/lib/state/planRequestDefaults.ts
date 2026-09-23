import type { PlanMode, PlanRequest } from "../api/types";

/** The `PlanMode` values `PlanRequest` can represent — everything except
 * `"delete"`, which is only ever produced internally by the folder-browser
 * delete flow and is never a user-selectable mode. */
export const PLAN_REQUEST_MODES = [
  "sort_by_type",
  "sort_by_date",
  "dedup",
  "cleanup",
] as const satisfies readonly PlanRequest["mode"][];

export function isPlanRequestMode(mode: PlanMode): mode is PlanRequest["mode"] {
  return (PLAN_REQUEST_MODES as readonly string[]).includes(mode);
}

export function defaultRequestForMode(mode: PlanRequest["mode"]): PlanRequest {
  switch (mode) {
    case "sort_by_type":
      return { mode };
    case "sort_by_date":
      return { mode, date_source: "modified", granularity: "year_month" };
    case "dedup":
      return { mode, min_size_bytes: 1024, keep_strategy: "oldest_modified" };
    case "cleanup":
      return {
        mode,
        stale_days: 180,
        date_source: "modified",
        action: "archive",
      };
  }
}
