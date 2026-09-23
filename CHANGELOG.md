# Changelog

Notable changes to sortty, newest first. This is the primary place to catch up on what changed without reading diffs — see `docs/adr/` for the reasoning behind the bigger decisions.

## 2026-09-24 — Progress/cancellation, empty trash, release CI & auto-update

Closes the three remaining 2026-09-23 architecture-review issues.

- **Scan and apply now report progress and can be cancelled.** Scanning shows a running "files found so far" count (a scan can't know the total upfront); applying shows a real "N of M operations" progress bar, since the operation count is known ahead of time. Cancelling an apply stops before the next file move (never mid-move) and keeps whatever was already applied — History now shows a "Cancelled" run instead of an unexplained partial one. See [ADR 0013](docs/adr/0013-scan-apply-progress-and-cancellation.md).
- **Added "Empty Trash" / "Empty Archive" to Browse** — the app's first genuinely irreversible action, permanently deleting a chosen folder's `.sortty-trash`/`.sortty-archive` staging folder and everything in it. Always shows a file-count/size preview first, and is skipped entirely with an info toast if there's nothing to empty. Does not appear in History — there's nothing to undo. See [ADR 0014](docs/adr/0014-empty-trash-scope-and-design.md).
- **Added CI-built, signed installers and in-app auto-update.** Pushing a version tag now builds and drafts a GitHub Release with signed Windows installers via `.github/workflows/release.yml`; a new "Check for updates" button in Settings lets installed copies check for and install the latest release. Update checks are manual only, by design — see [ADR 0012](docs/adr/0012-manual-update-checks.md). See [docs/RELEASING.md](docs/RELEASING.md) for the release process and one-time signing-key setup.

## 2026-09-24 — Follow-up fixes from the architecture review

Closed five of the eight issues the 2026-09-23 architecture review filed (the remaining three — progress/cancellation for long scans, release-CI automation, and the "empty the trash" feature — are larger feature/infra work left for their own passes).

- **Grid view is now virtualized**: the plan-preview table's thumbnail/grid view (the one 2D view the earlier virtualization pass explicitly skipped) now only renders tiles near the viewport via a new `VirtualGrid` component, matching list view's existing behavior for large folders. See [ADR 0009](docs/adr/0009-grid-virtualization-as-separate-component.md).
- **The app now remembers your last folder and mode**: `default_root`/`last_used_mode` were already round-tripped through Settings but never actually wired up — the last chosen folder and mode now silently restore on the next launch. No new setting to toggle; it just works, like the rest of the app's "remember X" behavior.
- **Verified long-path (MAX_PATH) and UNC-path handling** were flagged as untested, not necessarily broken — traced through the actual `std::path` semantics and added regression tests instead of speculative `\\?\` prefixing code. Both were already correct; see [ADR 0010](docs/adr/0010-verify-before-fixing-long-path-unc.md).
- **Added test coverage for the `commands/` (Tauri command) layer**, which previously had zero tests despite being the actual surface exposed to the frontend. Extracted each command's real logic into a plain, directly-testable function (`delete_files_at`, `undo_last_run_at`, `undo_run_at`, `scan_folder_at`, `read_file_preview_at`, `browse_folder_at`), matching the testable-core/thin-glue shape every other backend layer already uses. See [ADR 0011](docs/adr/0011-command-layer-testing-via-extracted-functions.md).
- **Accessibility**: the sidebar's current page is now announced via `aria-current="page"` (previously only a CSS class signaled it), and error toasts now use `role="alert"`/`aria-live="assertive"` instead of the same polite-only announcement every toast kind used before.

## 2026-09-23 — Architecture review: safety fixes, de-duplication, virtualization, CI

First full codebase/architecture review since the initial build. Fixed the real bugs it found, cleaned up duplication it flagged, and closed the process gaps (no CI, no lint, no LICENSE) — see [ADR 0007](docs/adr/0007-validate-configurable-folder-names.md) and [ADR 0008](docs/adr/0008-ci-and-lint-gates.md).

- **Fixed a real safety gap**: category names and the trash/archive folder names (both user-editable in Settings) were never validated before being joined onto the scan root, so a value like `"../../Desktop"` could move files outside the scanned folder — undercutting the core "nothing leaves root" promise. Rejected at save time now, with a plain-English error.
- **Fixed**: a crash mid-write to `settings.toml`, `categories.toml`, or the run-history index could leave a corrupted/truncated file behind, breaking History/Undo (or Settings) for every run, not just the one being saved. All config/history writes are now atomic (write-then-rename).
- **Fixed**: History and Settings could get stuck on their loading spinner forever, with no error shown, if the initial data load failed — both now show an error toast and recover.
- **Fixed**: the scanner aborted an entire scan if a single file couldn't be read (permission-denied, locked, a cloud-sync placeholder) — it now skips that one file and keeps going.
- Removed a panic risk in duplicate-detection's keeper selection, and switched it to hash files via a streaming reader instead of loading whole files into memory (relevant for large duplicate candidates like videos or disk images).
- De-duplicated the grouped-list logic that had drifted slightly between the plan-preview table and Browse (the exact kind of drift that already caused a shipped bug), and merged the two near-identical confirmation dialogs into one.
- The plan-preview table and Browse now virtualize their list view, so a folder with thousands of loose files no longer renders that many DOM rows at once. (Grid view isn't virtualized yet.)
- Added a starter frontend test suite (vitest) — previously there were zero frontend tests despite solid Rust coverage.
- Added ESLint + Prettier for the frontend, `rustfmt`/`clippy` gates for the backend, and a GitHub Actions CI pipeline running all of it (tests, type-checking, lint, build) on every push/PR to `main`.
- Added the missing `LICENSE` file (MIT, as `package.json` already declared).

## 2026-09-23 — Browse & delete sorted files, exclude subfolders from a scan, dark-mode native controls

- Added a new **Browse** page: pick any folder (typically a sorted destination like `Images/`), see everything currently in it, and delete files that don't belong. "Delete" moves files into a `.sortty-trash` folder next to them rather than deleting for real — it shows up in History and can be undone, exactly like a duplicate-cleanup run. See [ADR 0006](docs/adr/0006-browse-delete-reuses-move-to-trash.md).
- Added an "Exclude specific subfolders" list under "Include files in subfolders too" on Sort & Clean — lets you opt individual subfolders (e.g. an installer folder) out of a recursive scan, instead of only being able to turn recursion off entirely.
- **Fixed**: the scrollbar and number-input spinner arrows (Settings' "Stale after (days)" / "Minimum size (bytes)" fields) stayed light-themed in dark mode. These are native WebView controls that only follow the app's theme via the CSS `color-scheme` property, which was never set; added it alongside a themed scrollbar color.

## 2026-09-23 — Fix scan results lost when switching tabs, fix broken grid view

- Fixed: switching from Sort & Clean to History or Settings and back would silently discard the current scan/plan and selection. Splitting Sort & Clean into its own route unmounts the page on navigation, which was wiping its local state; that state now lives in a small module-level session object instead, so it survives switching tabs.
- Fixed: the plan-preview table's Grid view (added in the previous entry) didn't actually render anything — a scale-selector button list was rebuilt fresh on every render, which Svelte flagged as an unstable key and threw as a runtime error that broke the grid's rendering alongside it.

## 2026-09-23 — Window controls fix, theme switch, restore defaults, thumbnail preview

- **Fixed**: the custom title bar shipped in the UI modernization pass didn't actually work — you couldn't drag the window, and minimize/maximize/close silently did nothing. Tauri's permission system was blocking those window commands; `src-tauri/capabilities/default.json` now grants them explicitly. See the update note on [ADR 0005](docs/adr/0005-ui-modernization-stack.md).
- Added a manual theme switch (System / Light / Dark) in Settings, under a new "Appearance" section — the app still defaults to following the OS, but you can now override it.
- Added a "Restore defaults" button in Settings (with a confirmation dialog) that resets file type categories, cleanup thresholds, and trash/archive folder names back to the built-in defaults.
- The plan-preview table can now be switched between a compact list and a thumbnail grid (small/medium/large), with real image thumbnails for picture files and file-type icons for everything else. View and scale choices are remembered between sessions.

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
