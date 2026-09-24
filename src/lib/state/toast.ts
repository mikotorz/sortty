import { get, writable } from "svelte/store";

export type ToastKind = "success" | "error" | "info";

/** An optional button on a toast, e.g. "Scan again" after a stale-plan error. */
export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  action?: ToastAction;
}

export const toasts = writable<Toast[]>([]);

let nextId = 0;

export function pushToast(
  kind: ToastKind,
  message: string,
  duration = 4000,
  action?: ToastAction,
) {
  const id = nextId++;
  toasts.update((list) => [
    ...list,
    action ? { id, kind, message, action } : { id, kind, message },
  ]);
  setTimeout(() => dismissToast(id), duration);
}

export function dismissToast(id: number) {
  toasts.update((list) => list.filter((t) => t.id !== id));
}

/** Runs a toast's action and dismisses the toast. */
export function runToastAction(id: number) {
  const toast = get(toasts).find((t) => t.id === id);
  dismissToast(id);
  toast?.action?.run();
}
