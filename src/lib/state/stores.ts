import { writable } from "svelte/store";
import type { AppSettings } from "../api/types";

export const selectedRoot = writable<string | null>(null);

/** The last-loaded or last-saved `AppSettings`, so Sort & Clean can start
 * each mode from the defaults saved in Settings. `null` until loaded. */
export const appSettings = writable<AppSettings | null>(null);
