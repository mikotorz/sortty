# ADR 0001: Tauri (Rust) + SvelteKit for the desktop app

## Status

Accepted

## Context

Sortty needed to be a desktop GUI app that does real file-system work (recursive scanning, content hashing for duplicate detection, moving thousands of files) while staying approachable to build and maintain. The user wanted a GUI (not a CLI/script), was open to Rust specifically, and the app needed to feel native on Windows.

Options considered:

- **Electron** (Node + Chromium) — mature ecosystem, but ships a full Chromium runtime per app and keeps all file I/O in Node, which is a worse fit for CPU-bound work like parallel content hashing.
- **egui/eframe** (pure Rust, immediate-mode GUI) — single language, but the UI toolkit is utilitarian and would make the checkbox-heavy dry-run preview table more work to build well.
- **Tauri** (Rust backend + OS-native webview, HTML/CSS/JS or a light frontend framework for UI) — small binaries, native webview (no bundled Chromium), Rust handles all file I/O, and the UI can use ordinary web tooling.

Within Tauri, the frontend needed to render a reactive, checkbox-heavy preview table with grouping and live selection state. Svelte's compiler-based reactivity is a good fit for that without a virtual-DOM's overhead, and SvelteKit's static-adapter SPA mode is Tauri's documented pattern for this.

## Decision

Build on **Tauri v2** with a **SvelteKit + TypeScript** frontend (static adapter, SPA mode, `ssr = false`). All real file-system work (scanning, hashing, moving, journaling) lives in Rust behind `#[tauri::command]`s; the frontend only calls those commands and renders their results. The webview is granted no filesystem capability at all — file access happens exclusively through Rust commands.

## Consequences

- Small, fast-starting binaries with a native window; no bundled browser runtime.
- Content hashing for duplicate detection can use `rayon` for parallelism without touching the UI thread.
- The security boundary is simple to reason about: the webview can open a folder-picker dialog and call typed commands, and nothing else.
- Windows builds require the MSVC Rust toolchain — see [ADR 0004](0004-msvc-toolchain-required.md).
