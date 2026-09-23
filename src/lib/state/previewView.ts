import { writable } from "svelte/store";

export type ViewMode = "list" | "grid";
export type ThumbScale = "sm" | "md" | "lg";

function read<T extends string>(
  key: string,
  allowed: readonly T[],
  fallback: T,
): T {
  try {
    const v = localStorage.getItem(key);
    if (v && (allowed as readonly string[]).includes(v)) return v as T;
  } catch {
    // ignore, fall back to default
  }
  return fallback;
}

function persist(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // ignore write failures (e.g. storage disabled)
  }
}

export const previewViewMode = writable<ViewMode>(
  read("sortty-preview-view", ["list", "grid"] as const, "list"),
);
export const previewScale = writable<ThumbScale>(
  read("sortty-preview-scale", ["sm", "md", "lg"] as const, "md"),
);

previewViewMode.subscribe((v) => persist("sortty-preview-view", v));
previewScale.subscribe((v) => persist("sortty-preview-scale", v));

export const SCALE_PX: Record<ThumbScale, number> = { sm: 64, md: 96, lg: 144 };
