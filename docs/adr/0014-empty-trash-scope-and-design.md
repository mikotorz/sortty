# ADR 0014: Empty trash — scope, home, and confirmation design

## Status

Accepted

## Context

[Issue #6](https://github.com/mikotorz/sortty/issues/6) had no design decisions made — `CONTEXT.md` had flagged "empty the trash" as the one genuinely irreversible action the app doesn't have, since day one, needing its own scope call: what exactly gets emptied, whether it's scoped to one folder or global, where it lives in the UI, and how much confirmation friction it deserves.

## Decision

- **Empties both `.sortty-trash` and `.sortty-archive`**, as two separately-labeled actions ("Empty Trash" / "Empty Archive") sharing one backend command (`StagingKind::Trash`/`Archive`) and one frontend flow. They're mechanically identical (per [ADR 0002](0002-moves-not-deletes.md), both are already "move to a staging folder" mechanisms) but mean different things to a user — confirmed duplicates versus stale-but-maybe-still-wanted files — so collapsing them into one undifferentiated button risked someone clearing archived files while only meaning to clear confirmed duplicates.
- **Scoped per-root**, not global across every root History has ever touched. Every other action in the app (Sort & Clean, Browse) already operates on one chosen folder at a time; building a global "empty every `.sortty-trash` this app has ever seen" would have required new root-aggregation over History's flat run index that doesn't exist today, for a feature that isn't asking for it.
- **Lives in Browse**, tied to whichever folder Browse currently has open — the closest existing UI shape (a folder picker plus a confirm-before-acting flow) to what this needed.
- **Always shows a dry-run count/size preview before confirming**, matching the app's existing "preview before anything happens" philosophy used everywhere else (Sort & Clean's plan preview, the apply-confirmation dialog). If the preview finds nothing to delete, the confirm dialog is skipped entirely in favor of a plain "nothing to empty" toast.
- **Does not go through the RunRecord/History/Undo pipeline.** That pipeline's whole design — `AppliedOperation.from`/`to`, `undo::undo` replaying moves in reverse — assumes every recorded action is a reversible move. A real delete has no `from` to replay back to. Forcing it into that shape (e.g. a `RunRecord` with no meaningful undo path) would have made an already-well-understood invariant ("everything in History can be undone") suddenly have an exception. A toast on completion is sufficient.
- **Reuses `ConfirmModal` as-is, with stronger wording only** ("Unlike everything else in sortty, this cannot be undone") — no new danger-styling prop, no extra-friction interaction (e.g. typing the folder name to confirm). This matches the app's own precedent: Settings' "Restore defaults," its current sternest action, already just uses stronger copy on the same dialog shape rather than a visually distinct one.

## Consequences

- Reads the _actual configured_ trash/archive folder names (`AppSettings.trash.staging_folder_name`/`archive_folder_name`) rather than hardcoding `.sortty-trash`/`.sortty-archive` — unlike `commands::browse::delete_files_at` and `ScanOptions::default()`, which still hardcode the literals (a separate, pre-existing bug, not fixed here).
- `commands::trash::empty_staging_folder_at` guards `target.starts_with(root)` before calling `remove_dir_all` — cheap, and this is the one place in the codebase where a mistake is genuinely unrecoverable. This guard is best-effort, not a complete guarantee: a hand-edited `settings.toml` with a `..`-containing folder name could still lexically satisfy `starts_with` without the resolved path actually staying inside `root`. This mirrors [ADR 0007](0007-validate-configurable-folder-names.md)'s already-accepted gap (validation happens at save time, not at every read) — the attack surface is the user's own machine and their own config file.
- If a future need for global (all-roots) trash cleanup emerges, it can be layered on top of this per-root primitive rather than requiring a rewrite.
