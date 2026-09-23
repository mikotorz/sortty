# ADR 0015: Force a WebView2 bounds resync on every window resize

## Status

Accepted

## Context

Users reported that enlarging the app window left the UI clipped to the old
size — shrinking the window worked fine, but growing it did not cause the
content to fill the new area. This only reproduced in the built desktop app,
never in a plain browser tab pointed at the same Svelte frontend.

The frontend layout was ruled out: `+layout.svelte`'s app shell is plain CSS
flexbox (`height: 100vh`, no cached JS dimensions), and the virtualized
grid/list components track their container size via Svelte 5's
`bind:clientWidth`/`clientHeight`, which compiles to a real `ResizeObserver` —
symmetric on grow and shrink either way.

That leaves the native window shell. Sortty runs with `"decorations": false`
(a custom titlebar, see the UI modernization work), and `lib.rs` registered no
`.on_window_event` handler at all — resize was fully delegated to WebView2/wry's
default behavior. Tracing the vendored `wry`/`tao` source (pinned via
`Cargo.lock` to `tauri 2.11.6` / `wry 0.55.1` / `tao 0.35.3`) shows the
WebView2 child HWND's bounds are normally kept in sync by a `WM_SIZE`
window-subclass that calls `SetBounds`/`SetWindowPos`. On undecorated windows
on Windows, this sync is known to become unreliable specifically when growing:
shrinking clips the existing surface for free, but growing requires an active
repaint that doesn't always fire through the default path.

## Decision

`lib.rs`'s `tauri::Builder` chain now registers an `.on_window_event` handler
that, on every `WindowEvent::Resized`, re-asserts the window's size back to
itself (`window.set_size(*size)`) when running on Windows. This is a no-op
from the user's perspective (the size doesn't actually change) but nudges
wry/WebView2 to recompute and re-apply the webview's bounds, working around
the missed repaint.

## Consequences

- Fixes the clipped-UI-on-grow bug with a minimal, purely additive Rust change
  — no frontend changes, no new Cargo dependencies.
- The handler fires on every resize event (including the continuous stream
  during a drag-resize), each call cheap (a single `set_size` echo), so no
  meaningful performance cost is expected; this was smoke-tested with a rapid
  continuous drag-resize.
- If a future case shows the plain echo insufficient (Win32's `SetWindowPos`
  can no-op when the target size already equals the current size, so no new
  `WM_SIZE` would fire), the next step is a 1-pixel nudge-and-back guarded by
  a reentrancy flag — not implemented here since the plain echo resolved the
  reported bug.
- This is a workaround for upstream wry/tao/WebView2 behavior, not a sortty
  domain decision. If a future `tauri`/`wry` upgrade fixes the underlying
  resize sync, this handler becomes a harmless no-op and can be removed.
