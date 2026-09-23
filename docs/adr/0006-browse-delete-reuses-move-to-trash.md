# ADR 0006: "Delete" from Browse reuses Move-to-trash, not a new primitive

## Status

Accepted

## Context

The product owner asked for a way to delete files that already landed in a sorted destination folder (e.g. junk that ended up in `Images/`) without leaving the app. This is the first feature where the user directly asks for "delete" as a verb, which runs straight into [ADR 0002](0002-moves-not-deletes.md): sortty has exactly one primitive, `Operation` (always a move), and no delete anywhere in the codebase, specifically so apply and undo can share one code path.

Two options existed:
- **Add a real delete** (`fs::remove_file`, or the OS Recycle Bin via a crate like `trash`) — matches the word "delete" literally, but reintroduces the exact irreversibility problem ADR 0002 was written to avoid, and would need its own apply/undo/history handling parallel to the existing move-based one.
- **Keep it a move**, into a `.sortty-trash` folder next to the deleted file, exactly like Dedup already does for duplicates.

## Decision

"Delete" from the new Browse view (`commands::browse::delete_files`) builds ordinary `Operation`s with `OperationKind::MoveToTrash`, wraps them in a `Plan` tagged with a new `PlanMode::Delete`, and applies/persists them through the existing `apply::executor::apply` and `apply::store::save_run` — the same functions `apply_plan` uses for every other mode. `PlanMode::Delete` exists purely as a label for History display; nothing else matches on it exhaustively.

Browse itself (`commands::browse::browse_folder`) is not a new listing routine either — it calls the existing `engine::scanner::scan` against the folder the user picked, always recursively, always with `.sortty-trash`/`.sortty-archive` excluded, so a folder that's been sorted into over multiple runs (or has date-based subfolders from Sort by Date) is browsed in full.

## Consequences

- "Delete" gets undo, History visibility, and failure reporting for free — no new code paths, no new tests needed for those behaviors beyond the ones `apply`/`undo`/`store` already have.
- As with Dedup and Cleanup, this does **not** free disk space — files move into `.sortty-trash`, not away from the disk. The Browse UI and its confirmation dialog say "moved to trash" rather than "deleted," to avoid over-promising, consistent with the wording already used elsewhere in the app.
- A `.sortty-trash` folder can now be created inside an arbitrary browsed folder, not just at a scan root — this falls out naturally of computing the trash path as `source.parent()/.sortty-trash` rather than requiring a fixed root, and needs no special-casing because the scanner's existing name-based exclusion (`exclude`) already prunes any folder named `.sortty-trash` at any depth, so it won't reappear in a later Browse or scan.
