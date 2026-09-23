import { describe, expect, it, vi } from "vitest";
import { Channel, invoke } from "@tauri-apps/api/core";
import {
  applyPlan,
  browseFolder,
  cancelCurrentOperation,
  deleteFiles,
  generatePlan,
  getRun,
  saveCategoryRules,
  saveSettings,
  scanFolder,
  undoRun,
} from "./commands";
import type { AppSettings, CategoryRules, Plan } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  // A minimal stand-in for the real Channel class — generatePlan/applyPlan
  // construct one internally, so it needs to exist under this mock too.
  Channel: class Channel {
    onmessage: (data: unknown) => void = () => {};
  },
}));

const mockedInvoke = vi.mocked(invoke);

// These pin down the exact Tauri command name and argument shape each
// wrapper sends — the one place a typo here (or in the Rust #[tauri::command]
// signature) would otherwise only surface at runtime against the real app.
describe("commands.ts", () => {
  it("scanFolder invokes scan_folder with root and options", async () => {
    mockedInvoke.mockResolvedValueOnce({ entries: [] });
    await scanFolder("C:\\Downloads", {
      include_subfolders: true,
      exclude_folders: [],
    });
    expect(mockedInvoke).toHaveBeenCalledWith("scan_folder", {
      root: "C:\\Downloads",
      options: { include_subfolders: true, exclude_folders: [] },
    });
  });

  it("generatePlan invokes generate_plan with root, request, scanOptions, and a progress channel", async () => {
    mockedInvoke.mockResolvedValueOnce({} as Plan);
    await generatePlan("C:\\Downloads", { mode: "sort_by_type" });
    expect(mockedInvoke).toHaveBeenCalledWith("generate_plan", {
      root: "C:\\Downloads",
      request: { mode: "sort_by_type" },
      scanOptions: undefined,
      onProgress: expect.any(Channel),
    });
  });

  it("applyPlan invokes apply_plan with the plan id, selectedIds, and a progress channel", async () => {
    mockedInvoke.mockResolvedValueOnce({});
    await applyPlan("p1", ["op-1", "op-2"]);
    expect(mockedInvoke).toHaveBeenCalledWith("apply_plan", {
      planId: "p1",
      selectedIds: ["op-1", "op-2"],
      onProgress: expect.any(Channel),
    });
  });

  it("cancelCurrentOperation invokes cancel_current_operation with no args", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    await cancelCurrentOperation();
    expect(mockedInvoke).toHaveBeenCalledWith("cancel_current_operation");
  });

  it("getRun and undoRun pass runId through", async () => {
    mockedInvoke.mockResolvedValue({});
    await getRun("run-1");
    expect(mockedInvoke).toHaveBeenCalledWith("get_run", { runId: "run-1" });

    await undoRun("run-1");
    expect(mockedInvoke).toHaveBeenCalledWith("undo_run", { runId: "run-1" });
  });

  it("saveCategoryRules and saveSettings send the value under the field name Rust expects", async () => {
    mockedInvoke.mockResolvedValue(undefined);
    const rules = {
      categories: {},
      other_folder_name: "Other",
    } as CategoryRules;
    await saveCategoryRules(rules);
    expect(mockedInvoke).toHaveBeenCalledWith("save_category_rules", { rules });

    const settings = {} as AppSettings;
    await saveSettings(settings);
    expect(mockedInvoke).toHaveBeenCalledWith("save_settings", {
      settingsValue: settings,
    });
  });

  it("browseFolder and deleteFiles invoke with the expected shape", async () => {
    mockedInvoke.mockResolvedValue([]);
    await browseFolder("C:\\Downloads\\Images");
    expect(mockedInvoke).toHaveBeenCalledWith("browse_folder", {
      path: "C:\\Downloads\\Images",
    });

    mockedInvoke.mockResolvedValueOnce({});
    await deleteFiles("C:\\Downloads\\Images", ["a.png", "b.png"]);
    expect(mockedInvoke).toHaveBeenCalledWith("delete_files", {
      root: "C:\\Downloads\\Images",
      paths: ["a.png", "b.png"],
    });
  });
});
