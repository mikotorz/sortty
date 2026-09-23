<script lang="ts" generics="T">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { untrack, type Snippet } from "svelte";
  import { get } from "svelte/store";

  /**
   * Windows a potentially-huge flat list to the rows near the viewport, so a
   * folder with thousands of files doesn't render that many DOM rows at once.
   * Used for the (list-view) row lists in PreviewTable and BrowsePanel — the
   * two places a scan/browse result can genuinely get large.
   *
   * Virtualizes against a shared scroll container (`scrollElement`) rather
   * than one of its own, so multiple independent VirtualList instances (one
   * per collapsible directory group) can scroll together in a single
   * scrollbar. `scrollMargin` — this instance's own offset from the top of
   * `scrollElement`'s scrollable content — is measured from the DOM; bump
   * `layoutVersion` from the caller whenever nearby layout can have shifted
   * (a group above opened/closed, a sibling reflowed) to force a remeasure.
   */
  let {
    items,
    estimateSize,
    scrollElement,
    layoutVersion = 0,
    overscan = 8,
    row,
  }: {
    items: T[];
    estimateSize: number;
    scrollElement: HTMLElement | null;
    layoutVersion?: number;
    overscan?: number;
    row: Snippet<[T]>;
  } = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let scrollMargin = $state(0);

  const virtualizer = untrack(() =>
    createVirtualizer<HTMLElement, HTMLDivElement>({
      count: items.length,
      getScrollElement: () => scrollElement,
      estimateSize: () => estimateSize,
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

  // Reads `items.length`/`scrollMargin` reactively but `get()`s the
  // virtualizer without subscribing to it — `setOptions` writes back into
  // the same store, so a reactive `$virtualizer` read in here would
  // resubscribe this effect to its own write and loop forever.
  $effect(() => {
    const count = items.length;
    const margin = scrollMargin;
    get(virtualizer).setOptions({ count, scrollMargin: margin });
  });
</script>

<div bind:this={containerEl}>
  <div
    style="height: {$virtualizer.getTotalSize()}px; position: relative; width: 100%;"
  >
    {#each $virtualizer.getVirtualItems() as vi (vi.key)}
      <div
        style="position: absolute; top: 0; left: 0; width: 100%; height: {vi.size}px; transform: translateY({vi.start -
          scrollMargin}px);"
      >
        {@render row(items[vi.index])}
      </div>
    {/each}
  </div>
</div>
