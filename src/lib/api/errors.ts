/**
 * Errors from Tauri commands arrive as `{ kind, message }` (ADR 0019). `kind`
 * is the Rust `AppError` variant in snake_case; `message` is the same
 * plain-English text the app has always shown.
 */
export type AppErrorKind =
  | "io"
  | "not_found"
  | "not_a_directory"
  | "protected_path"
  | "protected_recursive"
  | "config"
  | "run_not_found"
  | "invalid_plan"
  | "already_undone"
  | "other";

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    typeof (e as AppError).kind === "string" &&
    typeof (e as AppError).message === "string"
  );
}

/** The human-readable message of anything a command (or JS) threw. */
export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

/** The backend error kind, or `null` for errors that didn't come from `AppError`. */
export function errorKind(e: unknown): AppErrorKind | null {
  return isAppError(e) ? e.kind : null;
}
