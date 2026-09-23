# ADR 0013: Progress and cancellation for scan/apply via Channel + an Ok-only cancel signal

## Status

Accepted

## Context

Scanning or applying a plan over a large folder blocked the UI with no feedback and no way to stop it ([issue #1](https://github.com/mikotorz/sortty/issues/1)). Fixing this touched two things that needed a real design call, not just an implementation:

1. **How progress gets from the backend to the frontend.** No event/channel mechanism existed anywhere in this codebase.
2. **How cancellation gets signaled back.** `scanner::scan`/`apply::executor::apply` are plain, Tauri-agnostic functions with their own unit tests calling them directly — a pattern used consistently everywhere in the backend and worth preserving rather than threading Tauri types into the engine layer.

## Decision

- **Progress**: `tauri::ipc::Channel<T>` as an extra parameter on the `generate_plan`/`apply_plan` commands, over `app.emit`/`listen`. It's typed, ordered, and scoped to one call — no global event-name string to keep in sync between backend and frontend. Sends are throttled to roughly every 50 items so a huge folder doesn't flood IPC with one message per file.
- **Engine functions stay Tauri-agnostic**: `scanner::scan_with_progress`/`executor::apply_with_progress` take plain `impl FnMut`/`impl Fn` closures for progress and cancellation, added alongside (not replacing) the existing `scan`/`apply`, which become thin wrappers passing no-op closures. This keeps their existing unit tests untouched and keeps the engine layer callable and testable without any Tauri context, consistent with every other module in `engine/`/`apply/`.
- **Cancellation is a single shared flag**, not per-request tokens: the UI already only ever allows one scan or one apply in flight at a time (the Scan/Apply buttons are disabled while busy), so an `Arc<AtomicBool>` in Tauri-managed state, reset at the start of each call, is enough.
- **Cancellation resolves through `Ok`, never a rejected promise.** `apply_plan` already returns a `RunRecord` — cancellation just means `record.cancelled == true`, with `applied_operations` holding whatever was actually moved (checked before each move, never mid-move, so nothing is left half-moved). `generate_plan`'s return type changed to `Result<Option<Plan>, AppError>`, where `Ok(None)` means "cancelled before a plan existed." No `AppError::Cancelled` variant was added — since `AppError` serializes to the frontend as a plain string (there's no structured "kind" that crosses the IPC boundary), string-matching a rejected promise to detect "the user cancelled" versus "something actually failed" would have been fragile in a way a wording change could silently break. Resolving through `Ok` sidesteps that entirely.

## Consequences

- Two new, slightly duplicated `_with_progress` functions exist alongside `scan`/`apply` rather than one function with optional callbacks — accepted, since it keeps the always-used no-progress path (all existing tests) completely unchanged.
- `RunRecord`/`RunSummary` gained a `cancelled: bool` field (`#[serde(default)]`, so runs persisted before this existed still deserialize) — History now distinguishes a cancelled run from a run that simply failed partway or had nothing to do.
- A scan can only report "N scanned so far," never "N of M" — `WalkDir`'s underlying walk is a lazy single-pass iterator with no upfront total. Apply, whose operation count is fixed ahead of time, gets a real "N of M" / determinate progress bar; scan gets an indeterminate spinner with a running count.
