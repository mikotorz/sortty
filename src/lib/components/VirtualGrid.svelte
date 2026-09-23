<script lang="ts" generics="T">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { untrack, type Snippet } from "svelte";
  import { get } from "svelte/store";

  /**
   * Windows a potentially-huge item list into a responsive tile grid, so a
   * folder with thousands of files doesn't render that many DOM tiles at
   * once in grid/thumbnail view. Virtualizes by row (not by tile) — column
   * count is derived from the measured container width, and each virtual
   * row renders exactly that many tiles via an explicit
   * `grid-template-columns`, so the JS chunking used to slice items into
   * rows always agrees with what the browser lays out (unlike CSS
   * `auto-fill`, which can't be trusted to match a row's fixed slice).
   *
   * Virtualizes against a shared scroll container (`scrollElement`) rather
   * than one of its own — see VirtualList for the shared-scroll-parent
   * rationale and the `scrollMargin`/`layoutVersion` contract.
   */
  let {
    items,
    minTileWidth,
    tileHeight,
    itemKey,
    gap = 12,
    scrollElement,
    layoutVersion = 0,
    overscan = 3,
    tile,
  }: {
    items: T[];
    minTileWidth: number;
    tileHeight: number;
    itemKey: (item: T) => string | number;
    gap?: number;
    scrollElement: HTMLDivElement | null;
    layoutVersion?: number;
    overscan?: number;
    tile: Snippet<[T]>;
  } = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let containerWidth = $state(0);
  let scrollMargin = $state(0);

  let columns = $derived(
    Math.max(1, Math.floor((containerWidth + gap) / (minTileWidth + gap))),
  );
  let rowCount = $derived(Math.ceil(items.length / columns));

  const virtualizer = untrack(() =>
    createVirtualizer<HTMLDivElement, HTMLDivElement>({
      count: rowCount,
      getScrollElement: () => scrollElement,
      estimateSize: () => tileHeight,
      overscan,
      scrollMargin: 0,
    }),
  );

  // Scroll-position-invariant: `scroller.scrollTop` cancels the rect delta,
  // so this is safe to (re)compute at any scroll position without listening
  // to scroll events — only on real layout changes.
  $effect(() => {
    void layoutVersion;
    const container = containerEl;
    const scroller = scrollElement;
    if (!container || !scroller) return;
    scrollMargin =
      container.getBoundingClientRect().top -
      scroller.getBoundingClientRect().top +
      scroller.scrollTop;
  });

  // Same pattern as VirtualList: read `rowCount`/`tileHeight`/`scrollMargin`
  // reactively but `get()` the virtualizer without subscribing to it, since
  // `setOptions` writes back into the same store a reactive `$virtualizer`
  // read would resubscribe this effect to.
  $effect(() => {
    const count = rowCount;
    const size = tileHeight;
    const margin = scrollMargin;
    get(virtualizer).setOptions({
      count,
      estimateSize: () => size,
      scrollMargin: margin,
    });
  });
</script>

<div bind:this={containerEl} bind:clientWidth={containerWidth}>
  <div
    style="height: {$virtualizer.getTotalSize()}px; position: relative; width: 100%;"
  >
    {#each $virtualizer.getVirtualItems() as vi (vi.key)}
      <div
        style="position: absolute; top: 0; left: 0; width: 100%; height: {vi.size}px; transform: translateY({vi.start -
          scrollMargin}px);
          display: grid; grid-template-columns: repeat({columns}, minmax(0, 1fr)); gap: {gap}px;"
      >
        {#each items.slice(vi.index * columns, vi.index * columns + columns) as item (itemKey(item))}
          {@render tile(item)}
        {/each}
      </div>
    {/each}
  </div>
</div>
