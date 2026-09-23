# ADR 0002: Everything is a Move — no real deletes, no Recycle Bin

## Status

Accepted

## Context

Two of the four modes (`Dedup`, `Cleanup`) conceptually "remove" files: a duplicate copy, or a stale file nobody's touched in months. A file-organizing tool that makes mistakes here is actively dangerous — false-positive duplicate detection or an overly aggressive stale-file threshold could destroy something the user needed.

Two conventional options existed for "removing" a file:

- **Send it to the OS Recycle Bin.** Familiar recovery UI, but Windows doesn't expose a simple, reliable Rust API for this — the available approaches are fragile (shell API quirks, no easy programmatic verification that the move succeeded) and it's a separate mental model from the "sort" operations, which just move files around normally.
- **Delete it for real** (`fs::remove_file`). Fastest to implement, but irreversible, and irreversible-by-default is the wrong posture for a tool whose entire job is bulk, semi-automated file operations.

## Decision

Sortty has exactly one primitive operation: **`Operation`**, which is always a move from `source` to `destination`. There is no delete anywhere in the codebase. What looks like "removing" a duplicate or stale file is a `Move` into a tool-owned staging folder inside the same root:

- Duplicates → `<root>/.sortty-trash/<relative-path>` (`OperationKind::MoveToTrash`)
- Stale files → `<root>/.sortty-archive/<relative-path>` (`OperationKind::Archive`)

Because every operation is a move, **undo is just replaying the same operations in reverse** (`apply/undo.rs`) — no separate "restore from trash" logic is needed. The only truly irreversible action in the product is a not-yet-built, explicitly-confirmed "empty the trash" feature.

## Consequences

- Apply and undo share one code path and one mental model (`apply/executor.rs` / `apply/undo.rs`), which made both far simpler to implement and test correctly than a delete-then-restore design would have been.
- Disk space isn't actually reclaimed by a dedup/cleanup run until the user (in a future release) empties `.sortty-trash`/`.sortty-archive` — the apply-result UI is careful to say "moved to trash (not yet permanently deleted)" rather than "freed," to avoid over-promising.
- `.sortty-trash` and `.sortty-archive` live inside the scanned root itself, so they're always excluded from future scans (`ScanOptions::exclude` defaults to both names) to avoid the tool re-processing its own staging folders.
