import { describe, expect, it } from "vitest";
import type { AppSettings } from "../api/types";
import { defaultRequestForMode } from "./planRequestDefaults";

const settings: AppSettings = {
  general: { default_root: null, last_used_mode: "sort_by_type" },
  sort_by_date: { granularity: "year", date_source: "created" },
  cleanup: { stale_days: 90, date_source: "created", action: "flag_only" },
  dedup: { keep_strategy: "shortest_path", min_size_bytes: 4096 },
  trash: {
    staging_folder_name: ".sortty-trash",
    archive_folder_name: ".sortty-archive",
  },
};

// Regression: the mode defaults saved in Settings were never used —
// Sort & Clean always started from hardcoded values.
describe("defaultRequestForMode", () => {
  it("uses the defaults saved in Settings", () => {
    expect(defaultRequestForMode("cleanup", settings)).toEqual({
      mode: "cleanup",
      stale_days: 90,
      date_source: "created",
      action: "flag_only",
    });
    expect(defaultRequestForMode("dedup", settings)).toEqual({
      mode: "dedup",
      min_size_bytes: 4096,
      keep_strategy: "shortest_path",
    });
    expect(defaultRequestForMode("sort_by_date", settings)).toEqual({
      mode: "sort_by_date",
      date_source: "created",
      granularity: "year",
    });
  });

  it("falls back to built-in defaults before settings load", () => {
    expect(defaultRequestForMode("cleanup")).toEqual({
      mode: "cleanup",
      stale_days: 180,
      date_source: "modified",
      action: "archive",
    });
  });
});
