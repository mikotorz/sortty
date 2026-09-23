# Sortty

A desktop app for sorting and cleaning up messy folders (Downloads, Desktop, etc.) — built with [Tauri](https://tauri.app) (Rust) and SvelteKit.

## What it does

Point it at a folder and pick a mode:

- **Sort by type** — group loose files into `Images/`, `Documents/`, `Videos/`, etc. based on extension (configurable in Settings).
- **Sort by date** — group loose files into `YYYY/MM` (or `YYYY`) folders based on modified/created date.
- **Find duplicates** — detect files with identical content (by hash) and move the extras aside.
- **Clean up stale files** — archive files that haven't been touched in a configurable number of days.

Every mode always shows a **dry-run preview** first — nothing moves until you review and confirm. "Deleting" a duplicate or stale file never really deletes it: it's moved into a `.sortty-trash` / `.sortty-archive` folder inside the scanned folder, and the whole run can be undone from the History tab.

The preview can be filtered by path, and viewed either as a list or as a thumbnail grid (with a small/medium/large size control) — image files show a real thumbnail, everything else shows a file-type icon. Both views only render the rows/tiles near the viewport, so a folder with thousands of files stays responsive. The app also remembers your last chosen folder and mode across restarts.

By default, only files sitting loose at the top level of the chosen folder are touched — existing subfolders (an installer's files, a driver package, a project folder) are left completely alone unless you explicitly opt in to scanning subfolders too. When subfolders are included, specific ones can be excluded by picking them from a list, so you can scan recursively without touching a folder you don't want reorganized.

The **Browse** page lets you look inside any folder — typically a sorted destination like `Images/` — and delete files that don't belong there. Like everything else, this doesn't really delete: files move into a `.sortty-trash` folder next to them, and the action can be undone from History.

## Tech stack

- **Backend**: Rust (Tauri v2) — all file scanning, categorization, and file I/O happens here.
- **Frontend**: SvelteKit + TypeScript, built as a static SPA served in Tauri's webview, styled with Tailwind CSS and [Bits UI](https://bits-ui.com/) (accessible unstyled component primitives). The window uses a custom frameless title bar. Theme follows the OS light/dark setting by default, with a manual Light/Dark/System switch in Settings.

See [CONTEXT.md](CONTEXT.md) for the domain model and [docs/adr/](docs/adr/) for key design decisions.

## Development

```bash
npm install
npm run tauri dev
```

Requires the Rust MSVC toolchain on Windows (`rustup toolchain install stable-x86_64-pc-windows-msvc` plus [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the C++ workload) — see [docs/adr/0004-msvc-toolchain-required.md](docs/adr/0004-msvc-toolchain-required.md) for why the GNU/MinGW toolchain doesn't work here.

### Tests, linting, and formatting

```bash
cd src-tauri && cargo test   # Rust unit/integration tests
cargo clippy && cargo fmt    # Rust lint / format (run from src-tauri/)
npm run check                # Svelte/TypeScript type-checking
npm test                     # frontend unit tests (vitest)
npm run lint                 # ESLint + Prettier check
npm run format                # Prettier --write
```

All of the above run in CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) on every push/PR to `main` — see [ADR 0008](docs/adr/0008-ci-and-lint-gates.md).

### Build

```bash
npm run tauri build
```

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
