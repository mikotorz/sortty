# ADR 0015: File lists share one scroll region instead of a fixed per-group height

## Status

Accepted

## Context

Users reported that enlarging the Sortty window left the file list looking
"stuck" — the window grew, but the list area didn't use the new space,
leaving blank background below it. A first attempt treated this as a native
WebView2 repaint bug on Windows (see the reverted `on_window_event` handler
this ADR's number briefly belonged to) — plausible in isolation, but a
screenshot from the user ruled it out: this was a CSS sizing issue, not a
stale render.

The real mechanism: `PreviewTable.svelte` and `BrowsePanel.svelte` render
scan/browse results as collapsible per-destination-directory groups, and each
group got its own independent `VirtualList`/`VirtualGrid` instance — each
capped at a hardcoded `max-height: 420px` with its own scrollbar. Nothing
above that in the DOM (`app-content` → page → `.card` → `Collapsible.Content`)
propagated a bounded height down; the whole page just scrolled normally. A
taller window therefore only added blank page background — the list itself
had no way to grow.

## Decision

Convert the two virtualized-list components to virtualize against one shared
scroll container per page, instead of each owning its own. `PreviewTable`
and `BrowsePanel` now wrap their `{#each groups}` block in a single
`flex-1 min-h-0 overflow-y-auto` div; the plan-preview/browse `.card` and its
page root propagate a bounded flex height down from `app-content` to that
div. Each group's `VirtualList`/`VirtualGrid` virtualizes against that shared
element using TanStack Virtual's `scrollMargin` option (the "window/shared
scroll parent" pattern), computing its own offset within the shared
scroller's content and remeasuring whenever a `ResizeObserver` on the groups
wrapper reports a layout shift (a group opening/closing, a grid reflowing its
column count).

The result: the whole list area — all groups together — now fills the
window and scrolls as one region with one scrollbar, the way a typical file
manager behaves, and groups still virtualize (don't render thousands of DOM
rows) even though they no longer scroll independently.

Other pages (Settings, History) are untouched — `app-content`'s page-level
`overflow-y: auto` still handles them exactly as before; only the two pages
with virtualized lists needed the flex/min-height:0 chain.

## Consequences

- Fixes the reported bug: the file list now visibly grows to fill the window.
- Adds real complexity to `VirtualList`/`VirtualGrid`: they no longer own
  their scroll element, and `scrollMargin` bookkeeping (measured via
  `getBoundingClientRect`, kept scroll-position-invariant by cancelling out
  `scrollTop`) is a non-obvious piece of code future readers will need this
  ADR to make sense of quickly.
- `PreviewTable`/`BrowsePanel` each own one `ResizeObserver` to detect layout
  shifts among sibling groups; this is a plain structural-change detector
  (bits-ui's `Collapsible.Content` toggles via an instant `display:none`, not
  an animation, so one observer firing per change is sufficient — no
  animation-frame polling needed).
- The `maxHeight` prop was removed from both components rather than kept as a
  fallback mode, since both of their only two call sites moved to the shared
  pattern — a dual-mode component would have been unused complexity.
- If a third caller ever needs a small, self-contained virtualized list (not
  sharing a page-level scroll region), it would need its own scroll wrapper
  reintroduced — not built preemptively here.

## Amendment (2026-09-24): minimum height for the results card

The first version gave the results card `min-height: 0` all the way down.
On Sort & Clean, the scan-options card above the results takes up most of a
default-size window, so the results card shrank to nearly zero and the list
couldn't be scrolled. The page column also has a fixed height, so the page
itself never overflowed and couldn't scroll either. The results card now has
a floor (`min-h-80`, 20rem): it still grows to fill a tall window, but in a
short one it keeps a usable height and the page-level `app-content` scroll
takes over, bringing the list into view.

## Amendment (2026-09-24): pages no longer have a fixed maximum width

The bug users actually saw was about width, not height. Every page's content
column had `max-w-3xl` (768px), so a wider window only added empty space to
the right of the cards. The layout changes above were still worth making,
because they let the list grow taller, but they didn't address this. The
list-oriented pages (Sort & Clean, Browse, History) now fill the available
width, like a file manager. Grid view adds tile columns to match (4 at the
default window size, 7 at 1600px wide). Settings keeps its 768px cap: it's
a form, and very wide text fields are harder to read.
