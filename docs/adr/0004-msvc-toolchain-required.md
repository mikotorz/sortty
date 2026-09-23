# ADR 0004: Windows builds require the MSVC Rust toolchain, not GNU/MinGW

## Status

Accepted

## Context

The development machine had no C++ linker installed at all — no Visual Studio Build Tools, no `rustup` (Rust came from a single Scoop-installed MSVC-targeted build). Installing Visual Studio Build Tools is a multi-gigabyte download, so a GNU/MinGW toolchain (via `rustup` + Scoop's `mingw`/`mingw-msvcrt` packages, targeting `x86_64-pc-windows-gnu`) was tried first as a lighter-weight alternative.

The GNU toolchain got the Rust code compiling — including working around a separate GNU-linker limitation where building this crate as a `cdylib` (only needed for mobile targets, which this app doesn't use) hit `ld`'s "export ordinal too large" limit on Tauri's very large transitive symbol table. Removing `cdylib` from `crate-type` (keeping `staticlib` + `rlib`) fixed that.

However, the resulting binary crashed on startup (`STATUS_ENTRYPOINT_NOT_FOUND`) before any code ran. The root cause: Tauri's dependency tree (via `tao`, `wry`, `webview2-com`, `tauri-plugin-single-instance`) pulls in **three incompatible versions of `windows-targets`** (0.42.2, 0.52.6, 0.53.5) simultaneously. On the GNU target, Windows API bindings are linked via raw symbol imports, and mixing multiple versions of that crate produces exactly this class of runtime crash. This is not a local misconfiguration — it's inherent to how the wider Tauri/`windows-rs` ecosystem currently depends on `windows-targets`, and is a documented reason the Rust/Windows community treats MSVC as the only reliable target for Tauri apps on Windows.

## Decision

Build and ship Sortty on Windows using the **MSVC** Rust toolchain (`stable-x86_64-pc-windows-msvc`), with Visual Studio Build Tools (C++ workload) installed. This is pinned per-project via [`src-tauri/rust-toolchain.toml`](../../src-tauri/rust-toolchain.toml) (`channel = "stable"`) plus a `rustup override` for the project directory, so the toolchain choice doesn't depend on whatever the machine's global default happens to be.

The GNU toolchain and MinGW packages that were installed while diagnosing this were removed afterward; they add no value once MSVC is confirmed working.

## Consequences

- Anyone building this project on Windows needs Visual Studio Build Tools (Desktop development with C++ workload) installed — there isn't a lighter-weight alternative that reliably works with Tauri today.
- `crate-type` in `src-tauri/Cargo.toml` stays `["staticlib", "rlib"]` (no `cdylib`) since this app is desktop-only; adding mobile targets later would need `cdylib` reintroduced, most likely per-platform.
- If the upstream `windows-targets` version conflict is ever resolved across Tauri's dependency tree, revisiting the GNU toolchain would be reasonable — but there's no reason to chase that proactively.
