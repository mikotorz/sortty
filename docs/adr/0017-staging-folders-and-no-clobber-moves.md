# ADR 0017: Staging folders are always excluded, and moves never overwrite

## Status

Accepted

## Context

[ADR 0002](0002-moves-not-deletes.md) promises that nothing is ever really deleted: every "removal" is a move into `.sortty-trash`/`.sortty-archive`. The 2026-09-24 review found two ways that promise was being broken in practice:

1. **Staged files could be sorted back out.** `ScanOptions::default()` excluded the two staging folders, but the frontend always sent its own `exclude: []`, which _replaced_ that default. So with "Include files in subfolders too" on, a scan went into `.sortty-trash`, and Sort by Type happily moved the duplicates that had been set aside back into `Images/`, `Documents/`, etc. The same happened to a trash folder renamed in Settings, because Browse and the scanner's default hard-coded the built-in names.
2. **A move could overwrite, or leave a stray copy.** `executor::move_file` used `std::fs::rename`, which on Windows (`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`) silently replaces an existing destination. The collision check just before it (`resolve_collision`) narrowed that to a race, but didn't close it. Worse, any rename error fell back to copy + remove. For a file that's open in another program the copy succeeds and the remove fails, leaving a second copy at the destination that no run record knows about, so Undo can't clean it up. `undo.rs` had its own copy of the same logic.

## Decision

- **Staging-folder exclusion lives in the backend and can't be overridden.** Every scan the command layer runs goes through `ScanOptions::excluding(trash.staging_folder_names())`. That list is the configured trash and archive names _plus_ the built-in defaults, so a folder staged under its old name stays protected after a rename. What the frontend sends in `exclude` is added to it, never used in its place; the TS `ScanOptions` type no longer has the field.
- **All file moves go through one no-clobber primitive**, `fsutil::move_no_clobber`, used by both apply and undo:
  - On Windows it calls `MoveFileExW` _without_ `MOVEFILE_REPLACE_EXISTING`, so an occupied destination is an error, never an overwrite.
  - It copies only on a genuine cross-volume error (`ERROR_NOT_SAME_DEVICE`), and that copy also refuses to overwrite.
  - If the source can't be removed after a cross-volume copy, the copy is deleted again and the move is reported as failed. The move either happens completely or not at all.

## Consequences

- A sharing-violation failure now leaves the filesystem exactly as it was, and shows up in the run's failures with the existing plain-English "this file is open in another program" message.
- If a name collision appears between planning and applying, the operation fails for that file instead of overwriting. `resolve_collision` still picks a free ` (n)` name first, so in practice this only matters in the race window.
- `windows-sys` becomes a direct dependency (it was already in the tree via Tauri).
