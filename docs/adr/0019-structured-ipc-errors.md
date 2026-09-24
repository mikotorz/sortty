# ADR 0019: Commands return errors as `{ kind, message }`

## Status

Accepted

## Context

`AppError` serialized to a plain string, so the webview could only print the message. It couldn't tell a stale preview from a permissions problem, and so couldn't offer the obvious next step: "Scan again" for a stale plan, or "Scan without subfolders" when the user folder was refused for a recursive scan. Matching on message text would break the first time a message was reworded.

## Decision

- Every command error serializes as `{ "kind": "<variant>", "message": "<text>" }`. `kind` is the `AppError` variant in snake_case (`AppError::kind()` in `src-tauri/src/error.rs`). `message` is the same `Display` text as before, so existing toasts read the same.
- The frontend reads errors only through `errorMessage(e)` and `errorKind(e)` (`src/lib/api/errors.ts`). These also accept plain strings and JS `Error`s, so a plugin or JS-side failure never renders as `[object Object]`.
- Kinds mean one thing each. "Run already undone" used to reuse `InvalidPlan`; it is now `AlreadyUndone`, so `invalid_plan` always means "this preview is out of date".
- A toast can carry one action button. That's how the app reacts to a kind: `invalid_plan` on Apply offers **Scan again**, and `protected_recursive` on Scan offers **Scan without subfolders**.

## Consequences

- Adding or renaming a variant is a change to the IPC contract. The Rust test `serializes_as_kind_and_message` and the TS `AppErrorKind` union must be updated together.
- Frontend code must not interpolate a caught error directly (`${e}`). Use `errorMessage(e)`.
