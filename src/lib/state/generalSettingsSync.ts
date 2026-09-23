import { getSettings, saveSettings } from "../api/commands";
import type { AppSettings } from "../api/types";

/**
 * Persists changes to `AppSettings.general` (the "remember last folder and
 * mode" fields) without clobbering the other settings sections, which are
 * loaded/saved as a whole elsewhere (e.g. SettingsPanel's save-all). Batches
 * bursts of changes into one debounced round trip, and re-reads the current
 * settings immediately before merging so a stale in-memory copy of the rest
 * of `AppSettings` is never written back.
 */
let pending: Partial<AppSettings["general"]> = {};
let timer: ReturnType<typeof setTimeout> | undefined;

export function scheduleGeneralSave(patch: Partial<AppSettings["general"]>) {
  pending = { ...pending, ...patch };
  clearTimeout(timer);
  timer = setTimeout(async () => {
    const toSave = pending;
    pending = {};
    const current = await getSettings();
    await saveSettings({
      ...current,
      general: { ...current.general, ...toSave },
    });
  }, 400);
}
