# ADR 0010: Verify long-path/UNC handling before adding speculative fixes

## Status

Accepted

## Context

The architecture review flagged two related, untested gaps: no `\\?\` long-path prefixing anywhere before filesystem calls in `apply/executor.rs`/`apply/undo.rs` (Windows' legacy `MAX_PATH` is 260 characters), and `scanner::is_protected_root`'s drive-root/top-level-folder logic being written and tested only against drive-letter paths (`C:\`, `C:\Windows`), never UNC paths (`\\server\share\...`). Neither had a single test proving current behavior one way or the other.

`\\?\` prefixing is not a free, purely-additive fix: it changes path semantics (relative paths and `.`/`..` normalization stop working under it), and the correct prefix form differs for local paths (`\\?\C:\...`) versus UNC paths (`\\?\UNC\server\share\...`) — using the wrong one, or double-prefixing an already-prefixed path, is exactly the kind of subtle bug this class of fix tends to introduce. Adding it speculatively, without first confirming the current code is actually broken, risks trading an unverified problem for a verified new one.

## Decision

Verify first, add tests either way, and only add fix code where a test actually fails.

- **UNC + `is_protected_root`**: traced through `std::path::Path`'s actual component semantics for a UNC path. `\\server\share` has no parent (`Path::parent()` returns `None`, the same as a drive root, since its only component after the UNC prefix is `RootDir`), and `\\server\share\Windows`'s parent's parent is likewise `None` with `file_name() == "Windows"` — so the existing drive-root logic already treats a UNC share root and its top-level folders exactly like a local drive root, correctly, with no code change. Added regression tests (`scanner.rs`) asserting this explicitly, since "traced through by hand once" isn't the same guarantee as "pinned by a test."
- **Long paths**: added regression tests in `executor.rs` and `undo.rs` that build a destination/source path deliberately over 260 characters inside a real temp directory and assert the move/restore still succeeds. These passed against the toolchain this app builds with — Rust's `std::fs` on Windows already handles paths past the legacy limit for the operations this codebase uses (`rename`, `create_dir_all`, `copy`, `remove_file`). No `\\?\` prefixing was added.
- Documented, but did not write, a fallback `long_path_safe()` helper for local-vs-UNC-aware prefixing, to be added only if a future toolchain or dependency change makes one of these regression tests start failing.

## Consequences

- Both concerns now have test coverage proving current behavior, closing the actual gap the review found (untested code, not necessarily broken code) without introducing prefixing logic that had no failing test to justify it.
- If a future Rust/Tauri upgrade changes Windows path handling, the new regression tests are the tripwire — a failure there is the trigger to implement the documented fallback, not a surprise found in production.
- This is a narrower fix than "add long-path support" would imply; the review's own framing ("verify... and fix whatever's actually broken") anticipated that the answer might be "nothing was broken."
