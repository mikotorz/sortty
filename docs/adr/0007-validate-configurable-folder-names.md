# ADR 0007: Validate user-configurable folder names before saving

## Status

Accepted

## Context

An architecture review of the whole codebase (the first since the initial build) found that category names and the trash/archive folder names — all user-editable via Settings, and persisted as plain strings in `categories.toml`/`settings.toml` — were never validated. The engine joins them straight onto `root` with `PathBuf::join` (`root.join(&category)` in `sort_by_type.rs`, `domain::plan::staged_destination` for dedup/cleanup), and `PathBuf::join` does not sanitize `..` components.

This is a real gap against [ADR 0002](0002-moves-not-deletes.md) and [ADR 0003](0003-non-recursive-scan-by-default.md)'s combined promise that sortty only ever moves files within the scanned root, into tool-owned staging folders. A `staging_folder_name` of `"../../Desktop"` (set via the Settings UI, or by hand-editing the TOML config in the OS config directory) would make dedup move "duplicates" outside the scanned root entirely — silently, since nothing checked for it. Windows-illegal characters and reserved device names (`CON`, `NUL`, `COM1`, ...) in a category name would similarly go unnoticed until the actual filesystem move failed with a confusing OS error.

## Decision

Added `domain::folder_name::validate_folder_name`, called from `config::settings::save_settings` (on `trash.staging_folder_name` and `trash.archive_folder_name`) and `config::settings::save_category_rules` (on every category name and `other_folder_name`) — at save time, in the config layer itself, not just in the Tauri command wrapper, so any future direct caller of these functions is protected too. It rejects: empty/whitespace-only names, `.` and `..`, any path separator (`/` or `\`), Windows-illegal characters (`< > : " | ? *`) and control characters, a trailing space or period, and the Windows-reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`9`, `LPT1`-`9`). A rejected name surfaces as a normal `AppError::Config` — the same plain-English error path Settings already uses for other save failures.

This is deliberately a save-time allowlist-by-rejection, not a defense added at every `PathBuf::join` call site — the engine's use of these strings (`sort_by_type.rs`, `domain::plan::staged_destination`) is unchanged and still trusts them, on the premise that a name that made it past `save_settings`/`save_category_rules` is safe for the lifetime of the config file.

## Consequences

- Setting a category or trash/archive folder name to anything resembling a path (via the UI or by hand-editing the TOML) is now rejected outright with a clear reason, instead of silently succeeding and only failing — or worse, quietly moving files outside `root` — on the next scan/apply.
- The validator lives in `domain/` (not `config/`) since it's a shared invariant, not config-file plumbing — `config::settings` and any future caller both depend on it, not the other way around.
- This does not protect a config file edited to bypass `save_*` entirely (e.g. a `categories.toml` hand-edited while the app isn't running) — `load_category_rules`/`load_settings` don't re-validate on read. That's an accepted gap for now: the attack surface is the user's own machine and their own config file, not an external input, so the highest-value place to catch a mistake is at the point they'd actually make one (Settings' Save button).
