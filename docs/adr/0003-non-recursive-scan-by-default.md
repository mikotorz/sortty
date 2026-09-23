# ADR 0003: Scans are non-recursive by default; drive roots and OS folders are refused

## Status

Accepted

## Context

Early versions of Sortty defaulted `ScanOptions::include_subfolders` to `true` — Sort by Type/Date walked every subfolder of the chosen root and reassigned files individually into category/date folders. Tried against a real Downloads folder, this reached into things like a "MATLAB installer" or "AMD drivers" folder — subfolders whose files need to stay together as a unit for the installer to work — and would have scattered their contents into `Images/`, `Documents/`, `Installers/`, etc. This was caught during manual testing, not by any automated check, which is itself a signal that the default was wrong rather than that a heuristic needed tuning.

Two narrower fixes were considered and rejected:

- **Auto-skip "bundle-like" folders** (heuristically detect subfolders that look like an app/installer bundle). Unpredictable — legitimate mixed-content folders could be misdetected either way, and a heuristic the user can't see or reason about is a worse UX than a plain default.
- **Manual exclude list** the user maintains in Settings. Puts the burden on the user to remember to exclude every such folder ahead of time, which fails silently the first time they forget.

Separately, nothing stopped a user from pointing the tool at `C:\` or `C:\Windows` — scanning would proceed and a plan could, in principle, propose moving core OS files.

## Decision

1. `ScanOptions::include_subfolders` defaults to `false` everywhere (Rust default, serde default, and the frontend's initial state). Only files sitting loose directly in the chosen root are ever scanned or sorted. An explicit, per-scan "Include files in subfolders too" checkbox opts in, with a warning shown when checked.
2. `scanner::is_protected_root` refuses to scan a drive root (a path with no parent) or a small deny-list of top-level OS folders directly under a drive root (`Windows`, `Program Files`, `Program Files (x86)`, `ProgramData`, `System Volume Information`, `Users`), returning `AppError::ProtectedPath` before any scanning happens.

Both checks live in `scan()` in [scanner.rs](../../src-tauri/src/engine/scanner.rs), so they protect every mode (`scan_folder` and `generate_plan` both call `scan()`) rather than needing to be duplicated per command.

## Consequences

- The common case — "declutter the loose files sitting in my Downloads" — now matches what actually happens by default; recursive scanning is available but is an informed, explicit choice.
- Dedup and Cleanup also default to top-level-only, which is a narrower duplicate/stale search by default than a power user doing a "deep clean" might want — the subfolder toggle covers that case.
- The protected-root deny-list is intentionally small and exact-match rather than a broad heuristic (e.g. it does not try to detect "looks like a system folder") — false negatives here are acceptable (the non-recursive default is the main safety net); false positives that block a legitimate folder are not.
