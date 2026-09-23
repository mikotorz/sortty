# Sortty — Domain Context

Sortty is a desktop tool that proposes and applies file-organizing operations against a folder the user picks (typically Downloads or Desktop). This document defines the vocabulary the codebase and its docs use consistently — use these terms, not synonyms, when writing issues, code, or design notes for this repo.

## Core model

- **Root** — the folder the user selected to scan. Never a drive root or a core OS folder (`Windows`, `Program Files`, etc.) — see [ADR 0003](docs/adr/0003-non-recursive-scan-by-default.md) and the `is_protected_root` guard in [scanner.rs](src-tauri/src/engine/scanner.rs).
- **Scan** — walking the root (and, only if opted in, its subfolders) to produce a list of `FileEntry` records: path, extension, size, modified/created timestamps. Certain files are never included in a scan — OS/browser bookkeeping files (`Thumbs.db`, `desktop.ini`, hidden/system-attribute files) and files that look like an in-progress download (`.crdownload`, `.part`, etc.). `ScanOptions.exclude_folders` lets the user opt specific subfolders (by full path) out of a recursive scan, on top of the always-applied name-based `exclude` used to skip `.sortty-trash`/`.sortty-archive`.
- **Mode** — which kind of organizing the user wants: `SortByType`, `SortByDate`, `Dedup` (find duplicates), or `Cleanup` (archive stale files). See `PlanMode` in [domain/plan.rs](src-tauri/src/domain/plan.rs).
- **Browse** — an ad hoc, unscanned-by-mode view into a single folder's _current_ contents (typically a sorted destination folder like `Images/`), used only to pick files for manual deletion. It reuses the ordinary scanner rather than a separate listing routine. See [ADR 0006](docs/adr/0006-browse-delete-reuses-move-to-trash.md).
- **Plan** — the _proposed_ set of `Operation`s for a mode, computed from a scan. A plan is pure data: generating one never touches the filesystem. This is what the dry-run preview renders.
- **Operation** — one proposed change: a `source` path, a `destination` path, a `reason` (shown in the preview, e.g. "Duplicate of ..."), and an `OperationKind`:
  - `Move` — reorganizing a file (sort by type/date).
  - `MoveToTrash` — a duplicate being set aside, into `.sortty-trash` inside the root.
  - `Archive` — a stale file being set aside, into `.sortty-archive` inside the root.
- **Run** — the record of actually _applying_ a plan (or a subset of its selected operations). A `RunRecord` captures which operations succeeded (`AppliedOperation`, with enough information to reverse it) and which failed (`FailedOperation`, with a plain-English `error`). Runs are persisted as JSON under the OS app-data directory so History and Undo survive restarts.
- **Undo** — reversing a run by replaying its `AppliedOperation`s in reverse (`to → from`). A path that's been reoccupied since the run is reported as a **conflict** rather than silently overwritten or aborting the rest of the undo. An undo that hits conflicts leaves the run **partially undone**: what could be restored is recorded (`restored_ids`), and the run can be retried until everything is back. Folders the run created and left empty are removed on undo.
- **Empty trash** — the one genuinely irreversible action in the app: permanently deleting everything inside a root's `.sortty-trash` or `.sortty-archive` staging folder (`std::fs::remove_dir_all`), scoped to one root at a time from Browse, always behind a dry-run count/size preview and a strongly-worded confirmation. Unlike every other action, it does not go through the Run/History/Undo pipeline — there is nothing to undo.

## The one invariant that matters: nothing is ever really deleted

Sortty never calls a real delete. "Removing" a duplicate or archiving a stale file both compile down to a `Move` into a tool-owned staging folder (`.sortty-trash` or `.sortty-archive`) inside the same root — never the Windows Recycle Bin, and never `fs::remove_file`. This is why apply/undo can be perfectly symmetric, and why every mode is safe to experiment with. See [ADR 0002](docs/adr/0002-moves-not-deletes.md).

Manually deleting a file from Browse follows the same rule: it's a `PlanMode::Delete` plan whose operations are ordinary `MoveToTrash` moves into the trash folder at the top of the browsed folder (keeping each file's relative path, the same layout Dedup uses), applied and recorded through the exact same `RunRecord`/History/Undo path as a Dedup run. See [ADR 0006](docs/adr/0006-browse-delete-reuses-move-to-trash.md).

The only genuinely irreversible action in the app's design is **emptying the trash** (see Empty trash above) — permanently deleting the `.sortty-trash`/`.sortty-archive` staging folders themselves, which necessarily falls outside the move-not-delete guarantee everything else in this section describes.

## Safety-by-default posture

Two defaults exist specifically because organizing tools are high blast-radius by nature:

1. **Scans are non-recursive by default.** Only loose files directly in the root are considered; existing subfolders (an installer's files, a driver package) are left alone unless the user explicitly opts in per-scan. See [ADR 0003](docs/adr/0003-non-recursive-scan-by-default.md).
2. **Drive roots and core OS folders are refused outright.** `is_protected_root` blocks scanning `C:\`, `C:\Windows`, `C:\Program Files`, etc., with a clear error rather than a partial, dangerous scan.

## Where things live

- Rust domain types: `src-tauri/src/domain/` (the nouns: `FileEntry`, `Plan`, `Operation`, `RunRecord`).
- Rust logic that turns a scan into a `Plan`: `src-tauri/src/engine/` (one file per mode).
- Rust logic that turns a `Plan` into filesystem changes: `src-tauri/src/apply/` (`executor.rs` applies, `undo.rs` reverses, `store.rs` persists runs).
- User-editable config (category → extension rules, stale-file threshold, trash/archive folder names): `src-tauri/src/config/settings.rs`, stored as TOML in the OS app-config directory.
- Frontend: `src/lib/components/` (one component per screen concern: `PreviewTable`, `ApplyConfirmModal`, `SettingsPanel`, `HistoryPanel`, `BrowsePanel`), talking to the Rust backend only through `src/lib/api/commands.ts`.
