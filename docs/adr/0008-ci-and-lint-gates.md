# ADR 0008: CI pipeline, and lint/format gates for both halves of the stack

## Status

Accepted

## Context

Through the UI modernization pass and the features that followed, sortty had real Rust test coverage (`cargo test`) and TypeScript type-checking (`npm run check`), but both were manual/local-only — nothing ran them automatically on push, despite the repo's `gh` token already having the `workflow` scope needed to add GitHub Actions. There was also no linting or formatting tooling on either side: no `rustfmt.toml`/`clippy` gate, no ESLint/Prettier for the frontend. A first full architecture review flagged this as a real gap: a regression in `commands/` (the Tauri command layer, which has zero test coverage of its own) or a frontend bug could land on `main` unnoticed.

## Decision

Added `.github/workflows/ci.yml` with two jobs:

- **`rust`**, on `windows-latest` (the app has `#[cfg(windows)]` code and, per [ADR 0004](0004-msvc-toolchain-required.md), needs the MSVC toolchain — it can't build on Linux/macOS runners at all): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **`frontend`**, on `ubuntu-latest` (none of its checks are Windows-specific, and Linux runners are faster/cheaper): `npm run check` (svelte-check), `npm run lint` (ESLint + Prettier), `npm test` (vitest), `npm run build`.

Both `-D warnings`/`--check` gates are hard fails, not advisory — this was only safe to do immediately because the existing trees were brought to a clean state first (one `cargo fmt` pass plus 3 pre-existing clippy fixes on the Rust side; one `prettier --write .` pass on the frontend, since neither had ever been run before). Had either tree had a large pre-existing backlog, the plan would have been to gate on a narrower allow-list and file a follow-up to clean up the rest — that wasn't needed here.

ESLint config (`eslint.config.js`, flat config) uses `typescript-eslint`'s recommended rules plus `eslint-plugin-svelte`'s recommended rules, with `eslint-config-prettier` last to disable any stylistic rules that would fight Prettier. `.svelte.ts` files (Svelte 5 runes used outside a component, e.g. `scanSession.svelte.ts`) needed an explicit override back to the plain TypeScript parser — `eslint-plugin-svelte`'s recommended config otherwise claims that glob for the Svelte parser, which doesn't handle a bare module with no template.

## Consequences

- A regression that breaks `cargo test`, introduces a clippy warning, fails `svelte-check`, fails lint, or breaks the vitest suite now fails CI on every push/PR to `main`, instead of only being caught if someone happens to run the check locally.
- Contributing a change now implies running `cargo fmt`/`npm run format` before committing, or CI will fail on formatting alone — this is a small ongoing tax in exchange for diffs that are never noise from mixed formatting.
- CI does not yet build/bundle the actual Tauri app (`tauri build`) or run it — it only compiles and tests the Rust lib and runs the frontend build (`vite build`/`svelte-kit`), which is cheaper and catches most regressions. Full installer builds on tag/release are explicitly out of scope for this change (tracked separately, not yet built).
- `cargo clippy`/`cargo fmt` gate on the full `src-tauri` crate; there's no separate CI job for `src-tauri/gen/` (Tauri-generated, gitignored) or any Rust code outside `src-tauri/`.
