# ADR 0018: A real content security policy

## Status

Accepted

## Context

`tauri.conf.json` had `"csp": null`, which means no content security policy at all. The window renders data straight from the filesystem: file names, paths, and SVG thumbnails. A CSP is the standard backstop against that data ever being treated as code. The app needs no network access from the webview (update checks run in Rust), and it loads nothing from outside its own bundle.

## Decision

```
default-src 'self';
img-src 'self' data:;
style-src 'self' 'unsafe-inline';
connect-src 'self' ipc: http://ipc.localhost
```

- `img-src data:` is for thumbnails, which arrive as data URLs.
- `style-src 'unsafe-inline'` is needed because Svelte and the virtualized lists set `style="…"` attributes at runtime (row positions, grid sizes, progress bar widths).
- `connect-src ipc: http://ipc.localhost` is Tauri's IPC transport.
- **`dangerousDisableAssetCspModification: ["style-src"]`.** At build time Tauri normally adds hashes of bundled inline styles to `style-src`. Once a hash is present, browsers ignore `'unsafe-inline'`, which would block every runtime `style` attribute and break the lists' layout. Script hashing stays on, so inline scripts are still hash-pinned.
- `devCsp: null`. In development the page is served by Vite with a hot-reload websocket, and applying the production policy there would only break development.

## Consequences

- Scripts can only come from the bundle, and nothing in the window can make network requests.
- Adding an external resource (a web font, a remote image) now needs an explicit CSP change, which is the point.
