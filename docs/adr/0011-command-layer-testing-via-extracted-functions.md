# ADR 0011: Test the commands/ layer by extracting plain functions, not by mocking Tauri

## Status

Accepted

## Context

The architecture review found all 15 `#[tauri::command]` functions under `src-tauri/src/commands/` had zero test coverage — the one layer in the backend that didn't follow the rest of the codebase's pattern of separating framework glue from testable logic (`engine/`, `apply/`, `config/`, `domain/` all take plain `&Path`/`&str` parameters and have `#[cfg(test)]` modules using `tempfile::tempdir()` directly). In every command, the only thing `AppHandle` is used for is resolving `app.path().app_data_dir()` / `app.path().app_config_dir()` into a `PathBuf` (plus one `app.opener()` call in `open_config_folder`) — the actual logic worth testing (`browse::delete_files`'s operation-building, `history::undo_last_run`'s "most recent undoable run" selection, etc.) never touches `AppHandle` itself.

Two approaches were available: add `tauri`'s `test` feature and use `tauri::test::mock_app()` to get a real (mocked) `AppHandle` pointed at a temp directory, or extract each command's logic into a plain function taking the resolved paths directly, leaving the `#[tauri::command]` as a thin `AppHandle → PathBuf` resolver.

## Decision

Extracted a plain, non-async, directly-testable function per command that has real logic (`delete_files_at`, `undo_last_run_at`, `undo_run_at`, `scan_folder_at`, `read_file_preview_at`, `browse_folder_at`), each taking `&Path` where the original took `AppHandle`-derived directories. The `#[tauri::command]` wrapper is now just: resolve `AppHandle` to paths, call the plain function, return its result. Tests call the plain functions directly with `tempfile::tempdir()`, exactly like every other layer already does.

Rejected `tauri::test::mock_app()` — it would introduce a second, heavier testing pattern for one layer, with an unverified payoff: whether tauri 2's mock app context lets you redirect `app_data_dir()`/`app_config_dir()` to a tempdir at all wasn't established, versus the extraction approach being a known-good pattern this codebase already uses everywhere else.

## Consequences

- `commands/` now follows the same testable-core/thin-glue shape as every other layer, instead of being the one exception.
- New commands should follow the same shape going forward: put real logic in a plain function taking resolved paths, keep the `#[tauri::command]` as a resolver-and-delegate. A command whose only real content is the resolution itself (e.g. `list_runs`, a one-line delegation to `store::list_runs`) doesn't need this extraction — it would just be testing the same thing `store.rs`'s own tests already cover.
- `history::undo_last_run_at`'s test coverage explicitly pins that "most recent" means "sorted newest-first by `store::list_runs`" — an implicit cross-module ordering guarantee the original code relied on silently.
- Priority coverage went to `browse::delete_files_at` and `history::undo_last_run_at` per the review's own prioritization; the three already-`AppHandle`-free commands (`scan_folder`, `read_file_preview`, `browse_folder`) were also extracted and tested since doing so needed no design decision, just the same mechanical split.
