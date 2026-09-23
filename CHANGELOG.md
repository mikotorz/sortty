# Changelog

Notable changes to sortty, newest first. This is the primary place to catch up on what changed without reading diffs — see `docs/adr/` for the reasoning behind the bigger decisions.

## 2026-09-23 — UI modernization

- Full visual and interaction redesign: Tailwind CSS v4 with a shared light/dark design-token system replaces the old scattered hardcoded colors, and Bits UI (accessible Svelte primitives) + lucide icons replace hand-rolled controls.
- The window now has a custom frameless title bar (app name + minimize/maximize/close) instead of the plain OS chrome, and navigation moved from top tabs to a left sidebar with real routes for Sort & Clean (`/`), History (`/history`), and Settings (`/settings`).
- The apply-confirmation dialog is now a proper accessible dialog (focus trap, Escape-to-close) instead of a hand-rolled modal.
- The plan-preview table gained a filter box (search by path) and collapsible per-destination groups, plus indeterminate-state checkboxes for "some selected" groups and a friendlier empty state.
- Scan/apply/save/undo feedback now goes through a small toast notification system instead of inline banners and a timed "Saved." message, with spinner icons on buttons while busy.
- Theming follows the OS light/dark setting automatically; there's no manual toggle by design. See [ADR 0005](docs/adr/0005-ui-modernization-stack.md) for the full reasoning and what was deliberately left out (e.g. native `<select>` elements were restyled rather than replaced, per-operation progress bars are still out of scope).

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
