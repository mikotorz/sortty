# ADR 0009: Grid (thumbnail) view gets its own virtualizer, not an extended VirtualList

## Status

Accepted

## Context

[ADR 0008](0008-ci-and-lint-gates.md)'s architecture review virtualized the plan-preview table and Browse's **list** view (`VirtualList.svelte`, wrapping `@tanstack/svelte-virtual`) but explicitly left the **grid** (thumbnail) view unvirtualized — it still rendered every tile via a plain `{#each}` over a CSS `grid-template-columns: repeat(auto-fill, minmax(...))` layout, so a folder with thousands of files still produced that many DOM tiles at once in grid view. The review flagged this as separate, meaningfully harder work, not a copy-paste of the list case: `VirtualList` windows a flat 1D list with a caller-supplied fixed row height; a grid needs to know how many tiles fit per row, and that number depends on the container's measured width (not fixed, since the grid is responsive) and the current thumbnail scale (`SCALE_PX`), which the user can change at runtime.

## Decision

Built a new `VirtualGrid.svelte`, separate from `VirtualList.svelte`, rather than extending `VirtualList` with an optional grid mode.

- `VirtualList` is also used by Browse's list view and the plan-preview table's list view. Bolting a grid/column concept onto its generic single-item-per-row API would bloat it for a feature only one caller (the grid branch) needs, and risks regressing the simpler, already-working list case.
- `VirtualGrid` virtualizes by **row**, not by individual tile: column count is derived from the scroll container's measured width (`bind:clientWidth`, reusing Svelte's built-in binding instead of a hand-rolled `ResizeObserver`) and the current minimum tile width, `columns = floor((containerWidth + gap) / (minTileWidth + gap))`. Each virtual row then slices exactly `columns` items out of the full list and renders them in a div with an **explicit** `grid-template-columns: repeat(columns, minmax(0, 1fr))` — not CSS `auto-fill`. Auto-fill lets the browser decide the column count independently of whatever count was used to slice items into that row's fixed-height box; if the two ever disagreed, tiles would silently wrap or overflow inside an absolutely-positioned row. Computing the column count once in JS and applying it identically to both the slicing and the CSS keeps them from being able to disagree.
- Tile height is a static value derived from the current thumbnail scale (`SCALE_PX[scale] + 62`), not measured — matching `VirtualList`'s own philosophy of a caller-supplied `estimateSize` rather than dynamic measurement per item.

## Consequences

- Grid view now only renders the tiles near the viewport, closing the gap left open by the original virtualization pass — consistent behavior with list view for large folders.
- Two virtualizer components exist side by side (`VirtualList`, `VirtualGrid`) instead of one generalized one. Accepted: they solve different layout problems (flat rows vs. a responsive multi-column grid), and keeping them separate keeps each one simple enough to read in one sitting.
- `VirtualGrid` briefly computes `columns` from a `containerWidth` of `0` before its first `bind:clientWidth` measurement lands (a one-frame layout settle). Not worth adding complexity to avoid — it self-corrects within the same paint cycle.
