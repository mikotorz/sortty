import { writable } from "svelte/store";

export type Theme = "system" | "light" | "dark";

const STORAGE_KEY = "sortty-theme";

function readStored(): Theme {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    // localStorage unavailable — fall back to system
  }
  return "system";
}

export const theme = writable<Theme>(readStored());

function applyTheme(value: Theme) {
  if (typeof document === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // ignore write failures (e.g. storage disabled)
  }
  if (value === "system") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", value);
  }
}

theme.subscribe(applyTheme);
