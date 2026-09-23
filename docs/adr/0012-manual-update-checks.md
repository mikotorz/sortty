# ADR 0012: Update checks are user-triggered, not automatic

## Status

Accepted

## Context

Adding auto-update (via `tauri-plugin-updater`, per [issue #7](https://github.com/mikotorz/sortty/issues/7), extended beyond its original CI-only scope at the product owner's request) raised a design choice the plugin itself doesn't force either way: should the app check for updates automatically (e.g. on every launch, silently in the background) or only when the user explicitly asks?

Nothing else in this codebase makes an unprompted network call. There's no telemetry, no crash reporting, no background polling of any kind — every network-adjacent action (scanning a folder, applying a plan, undoing a run) is local filesystem I/O triggered by an explicit user action. An automatic update check would be the first code path in the app that reaches out to the internet without the user having just clicked something asking it to.

## Decision

`SettingsPanel.svelte` gets a single "Check for updates" button that calls the updater plugin's `check()` on click. There is no check-on-launch, no periodic background check, and no setting to enable one.

## Consequences

- Consistent with the app's existing no-surprise-network-calls posture — a user who never opens Settings never causes sortty to talk to the network on its own.
- Users won't be nudged toward available updates proactively; they'll only see one if they think to check. This is an accepted tradeoff for a low-stakes desktop utility, not a security-critical always-connected app — revisit if update uptake turns out to matter more than expected.
- If automatic checking is wanted later, it's an additive change (a call to `check()` in `+layout.svelte`'s existing `onMount`, alongside the settings-hydration logic already there) — this decision doesn't foreclose it, it just isn't the default.
