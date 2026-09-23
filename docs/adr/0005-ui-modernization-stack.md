# ADR 0005: UI modernization stack — Tailwind CSS v4, Bits UI, custom title bar, sidebar nav

## Status

Accepted

## Context

The original frontend had no CSS framework: every component carried its own `<style>` block, colors were hardcoded hex repeated across files (`#396cd8` in four-plus places), dark mode only swapped three CSS variables and left most components with hardcoded light-mode fallbacks (`var(--border-color, #ccc)`), and the app used the plain OS-default Windows title bar with a top tab bar for navigation. The hand-rolled `ApplyConfirmModal` had no focus trap or Escape-to-close. None of this was a functional bug, but it read as dated, and the preview table (the main workflow surface) had no way to search or collapse a long list of proposed operations.

The product owner asked for a full visual and interaction modernization: a clean/minimal look, automatic OS light/dark theming (no manual toggle), a custom frameless title bar, a left sidebar, and a substantially more usable preview table.

## Decision

- **Tailwind CSS v4** (via `@tailwindcss/vite`) for styling, replacing scattered per-component `<style>` blocks with utility classes plus a small set of shared component classes (`.card`, `.btn-primary`, `.btn-ghost`, `.field`) in `src/app.css`.
- **Design tokens as CSS custom properties inside `@theme`**, so Tailwind generates utilities (`bg-accent`, `text-muted`, etc.) directly from them, and a single `@media (prefers-color-scheme: dark)` block overrides those same properties for dark mode — no manual theme toggle, no `class`-based dark mode, no extra runtime dependency (e.g. `mode-watcher`) for something that isn't in scope.
- **Bits UI** (unstyled, accessible Svelte 5 primitives) for `Dialog` (replacing the hand-rolled modal — fixes focus trap, Escape-to-close, and scroll lock for free), `Collapsible` (per-destination groups in the preview table), and `Checkbox` (adds a proper indeterminate state for "some selected" group checkboxes, which a native checkbox can't express without manual DOM manipulation).
- **`@lucide/svelte`** for icons — tree-shakeable per-icon imports, no icon font or CSS-in-JS baggage, pairs naturally with a "bring your own styling" primitives library.
- **Custom frameless title bar** (`"decorations": false` in `tauri.conf.json`, hand-drawn `TitleBar.svelte` using `@tauri-apps/api/window`'s `getCurrentWindow()`) instead of the native OS chrome — the single biggest visual signal that this is a considered desktop app rather than a template scaffold.
- **Left sidebar navigation**, and the three sections (Sort & Clean / History / Settings) became real SvelteKit routes (`/`, `/history`, `/settings`) instead of an in-memory tab variable — idiomatic once a persistent `+layout.svelte` shell exists, and gives URL/back-forward behavior for free without any adapter changes (the existing `adapter-static` SPA-fallback setup already serves any path client-side).
- Native `<select>` elements were kept (restyled with Tailwind) rather than replaced with Bits UI's `Select` primitive as originally considered — the six existing selects rely on Svelte's native typed `bind:value` coercion, and porting them to a primitive whose value model is string-only would have added real complexity for a purely cosmetic gap (native selects were already keyboard-accessible; they just couldn't be restyled consistently, which plain CSS `appearance` overrides address well enough).

## Consequences

- `src/lib/state/stores.ts` was pruned to just `selectedRoot` — the other stores it defined (`currentPlan`, `selectedOperationIds`, `busy`, `statusMessage`, `lastRunRecord`, `currentView`) duplicated state each page already tracked locally via Svelte 5 runes and were unused; keeping both was a latent two-sources-of-truth risk, not a real requirement.
- A lightweight custom toast store (`src/lib/state/toast.ts`) replaced the old ad hoc inline banners and `setTimeout`-cleared save message — no toast library dependency was justified for two async actions (scan, apply) plus settings-save and undo.
- True per-operation progress bars during scan/apply would require the Rust side to emit Tauri events during the loop; that's a backend change and stayed out of scope for this frontend-only pass. If large-folder scans start feeling unresponsive, that's a separate follow-up.
- A frameless window loses the OS's default Snap Layouts affordance on Windows 11 unless explicitly reimplemented; this wasn't done, since Tauri v2 doesn't restore it automatically for custom-chrome windows. Worth revisiting if it becomes an actual complaint.
