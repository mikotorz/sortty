# Changelog

Notable changes to sortty, newest first. This is the primary place to catch up on what changed without reading diffs — see `docs/adr/` for the reasoning behind the bigger decisions.

## 2026-09-23 — Documentation & repo setup

- Published the project to a private GitHub repo: [mikotorz/sortty](https://github.com/mikotorz/sortty).
- Added `CONTEXT.md` (domain glossary: Root, Scan, Plan, Operation, Run, Undo) and four ADRs documenting the Tauri/Svelte stack choice, the move-not-delete design, the non-recursive-scan safety fix, and the MSVC toolchain requirement.
- Rewrote `README.md` with real project info (previously the default Tauri template text).
- Added a "Working style" section to `CLAUDE.md`: the user gives feature requests and reviews; code and documentation are written proactively.

## 2026-09-23 — Safety & polish pass

- Scans are now **non-recursive by default** — only loose top-level files are touched; existing subfolders (installer folders, driver packages, etc.) are left alone unless explicitly opted into via a new "include subfolders" toggle. This was the fix for a real bug: sorting a real Downloads folder would have scattered files out of things like a "MATLAB installer" folder.
- Scanning a drive root or core OS folder (`C:\`, `C:\Windows`, etc.) is now refused outright with a clear error.
- In-progress downloads (`.crdownload`, `.part`, etc.) and OS/junk files (`Thumbs.db`, `desktop.ini`, hidden/system-attribute files) are now always skipped rather than swept into "Other".
- Failure messages are now plain-English (e.g. "this file is open in another program") instead of raw OS error codes, visible in both the apply-result banner and History ("why?" link per run).
- The apply-result banner now shows how much was moved to trash / archived after a run.

## 2026-09-23 — Initial build

- First working version: Tauri (Rust) + SvelteKit desktop app with four modes — Sort by Type, Sort by Date, Find Duplicates, Clean Up Stale Files — always behind a dry-run preview that requires explicit confirmation before anything moves.
- Move-only model: nothing is ever permanently deleted. Duplicates and stale files move into `.sortty-trash` / `.sortty-archive` inside the scanned folder, and any run can be undone from History.
- Configurable file-type categories and stale-file threshold via Settings, backed by TOML config files.
- 19 Rust unit/integration tests covering categorization, scanning, sorting, dedup, cleanup boundaries, apply/collision-handling, and undo/conflict-handling.
