<script lang="ts" generics="T">
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { untrack, type Snippet } from "svelte";
  import { get } from "svelte/store";

  /**
   * Windows a potentially-huge flat list to the rows near the viewport, so a
   * folder with thousands of files doesn't render that many DOM rows at once.
   * Used for the (list-view) row lists in PreviewTable and BrowsePanel — the
   * two places a scan/browse result can genuinely get large.
   */
  let {
    items,
    estimateSize,
    maxHeight = 420,
    overscan = 8,
    row,
  }: {
    items: T[];
    estimateSize: number;
    maxHeight?: number;
    overscan?: number;
    row: Snippet<[T]>;
  } = $props();

  let scrollEl = $state<HTMLDivElement | null>(null);

  const virtualizer = untrack(() =>
    createVirtualizer<HTMLDivElement, HTMLDivElement>({
      count: items.length,
      getScrollElement: () => scrollEl,
      estimateSize: () => estimateSize,
      overscan,
    }),
  );

  // Reads `items.length` reactively but `get()`s the virtualizer without
  // subscribing to it — `setOptions` writes back into the same store, so a
  // reactive `$virtualizer` read in here would resubscribe this effect to
  // its own write and loop forever.
  $effect(() => {
    const count = items.length;
    get(virtualizer).setOptions({ count });
  });
</script>

<div bind:this={scrollEl} class="overflow-y-auto" style="max-height: {maxHeight}px;">
  <div style="height: {$virtualizer.getTotalSize()}px; position: relative; width: 100%;">
    {#each $virtualizer.getVirtualItems() as vi (vi.key)}
      <div
        style="position: absolute; top: 0; left: 0; width: 100%; height: {vi.size}px; transform: translateY({vi.start}px);"
      >
        {@render row(items[vi.index])}
      </div>
    {/each}
  </div>
</div>
