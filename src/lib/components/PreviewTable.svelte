<script lang="ts">
  import { Collapsible, Checkbox } from "bits-ui";
  import type { Operation, Plan } from "../api/types";
  import { formatBytes } from "../format";
  import { cn } from "../cn";
  import { getScrollRoot } from "../state/scrollRoot.svelte";
  import {
    previewViewMode,
    previewScale,
    SCALE_PX,
    type ThumbScale,
  } from "../state/previewView";
  import {
    dirOf,
    fileNameOf,
    groupByDir,
    groupByDuplicateSet,
    isGroupChecked,
    isGroupIndeterminate,
    withGroupSelection,
  } from "../grouping";
  import FileThumb from "./FileThumb.svelte";
  import VirtualList from "./VirtualList.svelte";
  import VirtualGrid from "./VirtualGrid.svelte";
  import Search from "@lucide/svelte/icons/search";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Check from "@lucide/svelte/icons/check";
  import Minus from "@lucide/svelte/icons/minus";
  import FolderCheck from "@lucide/svelte/icons/folder-check";
  import List from "@lucide/svelte/icons/list";
  import LayoutGrid from "@lucide/svelte/icons/layout-grid";

  let {
    plan,
    selected = $bindable(),
    onChooseKeeper,
    choosingKeeper = false,
  }: {
    plan: Plan;
    selected: Record<string, boolean>;
    /** Find Duplicates: keep this copy instead of its set's keeper. */
    onChooseKeeper?: (operationId: string) => void;
    choosingKeeper?: boolean;
  } = $props();

  const SCALE_OPTIONS: [ThumbScale, string][] = [
    ["sm", "S"],
    ["md", "M"],
    ["lg", "L"],
  ];

  const idOf = (op: Operation) => op.id;

  let query = $state("");
  let openGroups = $state<Record<string, boolean>>({});

  const scrollRoot = getScrollRoot();

  let filteredOps = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return plan.operations;
    return plan.operations.filter(
      (op) =>
        op.source.toLowerCase().includes(q) ||
        op.destination.toLowerCase().includes(q),
    );
  });

  interface Group {
    key: string;
    /** The destination folder, or for Find Duplicates the kept file. */
    title: string;
    /** Find Duplicates only: the folder of the kept file. */
    keeperDir?: string;
    ops: Operation[];
  }

  // Find Duplicates groups each keeper with its copies (so the user can
  // pick a different keeper); every other mode groups by destination folder.
  let isDedup = $derived(
    plan.mode === "dedup" && (plan.duplicate_sets?.length ?? 0) > 0,
  );

  let groups = $derived.by((): Group[] => {
    if (isDedup) {
      return groupByDuplicateSet(plan.duplicate_sets ?? [], filteredOps).map(
        ({ set, copies }) => ({
          key: set.id,
          title: fileNameOf(set.keeper),
          keeperDir: dirOf(set.keeper),
          ops: copies,
        }),
      );
    }
    return groupByDir(filteredOps, (op) => op.destination).map(
      ([dir, ops]) => ({ key: dir, title: dir, ops }),
    );
  });

  function isGroupOpen(dir: string): boolean {
    return openGroups[dir] ?? true;
  }

  let allSelected = $derived(isGroupChecked(plan.operations, idOf, selected));
  let someSelected = $derived(
    isGroupIndeterminate(plan.operations, idOf, selected),
  );

  function toggleAll(value: boolean) {
    selected = withGroupSelection(plan.operations, idOf, selected, value);
  }

  function toggleGroup(ops: Operation[], value: boolean) {
    selected = withGroupSelection(ops, idOf, selected, value);
  }
</script>

<div
  class="flex items-center justify-between border-b border-[var(--color-border)] pb-2.5 mb-3 gap-3"
>
  <label class="flex items-center gap-2 text-sm font-medium">
    <Checkbox.Root
      checked={allSelected}
      indeterminate={someSelected}
      onCheckedChange={(v) => toggleAll(v === true)}
      class="chk"
    >
      {#snippet children({ checked, indeterminate })}
        {#if indeterminate}<Minus size={11} />{:else if checked}<Check
            size={11}
          />{/if}
      {/snippet}
    </Checkbox.Root>
    Select all ({plan.summary.total_files} files, {formatBytes(
      plan.summary.total_bytes,
    )})
  </label>

  <div class="flex items-center gap-3">
    {#if $previewViewMode === "grid"}
      <div
        class="inline-flex rounded-md border border-[var(--color-border)] p-0.5 text-xs"
      >
        {#each SCALE_OPTIONS as [value, label] (value)}
          <button
            type="button"
            class={cn(
              "rounded px-2 py-1",
              $previewScale === value
                ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]"
                : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
            )}
            onclick={() => previewScale.set(value)}
          >
            {label}
          </button>
        {/each}
      </div>
    {/if}
    <div
      class="inline-flex rounded-md border border-[var(--color-border)] p-0.5"
    >
      <button
        type="button"
        aria-label="List view"
        class={cn(
          "flex items-center rounded p-1.5",
          $previewViewMode === "list"
            ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]"
            : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
        )}
        onclick={() => previewViewMode.set("list")}
      >
        <List size={14} />
      </button>
      <button
        type="button"
        aria-label="Grid view"
        class={cn(
          "flex items-center rounded p-1.5",
          $previewViewMode === "grid"
            ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]"
            : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
        )}
        onclick={() => previewViewMode.set("grid")}
      >
        <LayoutGrid size={14} />
      </button>
    </div>
    <div class="relative">
      <Search
        size={14}
        class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--color-text-muted)]"
      />
      <input
        type="text"
        placeholder="Filter by path…"
        bind:value={query}
        class="w-56 rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] pl-8 pr-2.5 py-1.5 text-sm outline-none focus:border-[var(--color-accent)]"
      />
    </div>
  </div>
</div>

{#if plan.operations.length === 0}
  <div
    class="flex flex-col items-center gap-2 py-10 text-[var(--color-text-muted)]"
  >
    <FolderCheck size={28} />
    <p class="m-0 text-sm">
      Nothing to do — this folder already looks tidy for this mode.
    </p>
  </div>
{:else if groups.length === 0}
  <p class="py-6 text-sm text-[var(--color-text-muted)]">
    No operations match "{query}".
  </p>
{/if}

{#snippet tileButton(op: Operation)}
  <button
    type="button"
    class={cn(
      "relative flex w-full flex-col items-center gap-1.5 rounded-lg border p-2.5 text-center",
      selected[op.id]
        ? "border-[var(--color-accent)] bg-[var(--color-surface-hover)]"
        : "border-transparent hover:bg-[var(--color-surface-hover)]",
    )}
    onclick={() => (selected = { ...selected, [op.id]: !selected[op.id] })}
  >
    <div
      class="chk absolute left-1.5 top-1.5"
      data-state={selected[op.id] ? "checked" : "unchecked"}
      aria-hidden="true"
    >
      {#if selected[op.id]}<Check size={11} />{/if}
    </div>
    <FileThumb path={op.source} size={SCALE_PX[$previewScale]} />
    <span class="w-full truncate text-xs" title={op.source}
      >{fileNameOf(op.source)}</span
    >
    <span class="text-[0.7rem] text-[var(--color-text-muted)]"
      >{formatBytes(op.size_bytes)}</span
    >
  </button>
{/snippet}

{#snippet keepButton(op: Operation)}
  <button
    type="button"
    class="btn-ghost whitespace-nowrap px-2 py-0.5 text-xs"
    disabled={choosingKeeper}
    title="Keep this copy and move the currently kept file to the trash instead"
    onclick={() => onChooseKeeper?.(op.id)}
  >
    Keep this one
  </button>
{/snippet}

{#each groups as { key, title, keeperDir, ops } (key)}
  <Collapsible.Root
    class="group mb-2 rounded-md border border-[var(--color-border-subtle)] overflow-hidden"
    open={isGroupOpen(key)}
    onOpenChange={(v) => (openGroups = { ...openGroups, [key]: v })}
  >
    <div
      class="flex items-center gap-2 bg-[var(--color-surface-hover)] px-2.5 py-1.5"
    >
      <Checkbox.Root
        checked={isGroupChecked(ops, idOf, selected)}
        indeterminate={isGroupIndeterminate(ops, idOf, selected)}
        onCheckedChange={(v) => toggleGroup(ops, v === true)}
        class="chk"
      >
        {#snippet children({ checked, indeterminate })}
          {#if indeterminate}<Minus size={11} />{:else if checked}<Check
              size={11}
            />{/if}
        {/snippet}
      </Checkbox.Root>
      <Collapsible.Trigger
        class="group-trigger flex flex-1 items-center gap-1.5 text-left text-sm"
      >
        <ChevronRight
          size={14}
          class="chevron shrink-0 text-[var(--color-text-muted)]"
        />
        {#if keeperDir !== undefined}
          <span class="text-xs text-[var(--color-text-muted)]">Keeping</span>
          <code
            class="shrink-0 whitespace-nowrap text-[0.8rem]"
            title="{keeperDir}\{title}">{title}</code
          >
          <span
            class="min-w-0 truncate text-xs text-[var(--color-text-muted)]"
            title={keeperDir}>in {keeperDir}</span
          >
          <span class="shrink-0 text-xs text-[var(--color-text-muted)]"
            >— {ops.length} {ops.length === 1 ? "copy" : "copies"}</span
          >
        {:else}
          <code class="text-[0.8rem]">{title}</code>
          <span class="text-xs text-[var(--color-text-muted)]"
            >({ops.length})</span
          >
        {/if}
      </Collapsible.Trigger>
    </div>
    <Collapsible.Content>
      {#if $previewViewMode === "list"}
        <div class="text-sm" role="table">
          <VirtualList
            items={ops}
            estimateSize={37}
            scrollElement={scrollRoot.el}
            layoutVersion={scrollRoot.layoutVersion}
          >
            {#snippet row(op: Operation)}
              <div
                role="row"
                class="grid h-full items-center gap-2.5 border-t border-[var(--color-border-subtle)] px-2.5 hover:bg-[var(--color-surface-hover)]"
                style="grid-template-columns: {isDedup
                  ? '2rem minmax(0, 1fr) auto auto'
                  : '2rem minmax(0, 55%) minmax(0, 1fr) auto'};"
              >
                <Checkbox.Root
                  checked={selected[op.id]}
                  onCheckedChange={(v) =>
                    (selected = { ...selected, [op.id]: v === true })}
                  class="chk"
                >
                  {#snippet children({ checked })}
                    {#if checked}<Check size={11} />{/if}
                  {/snippet}
                </Checkbox.Root>
                <span
                  class="overflow-hidden text-ellipsis whitespace-nowrap"
                  title={op.source}
                >
                  {op.source}
                </span>
                {#if isDedup}
                  {@render keepButton(op)}
                {:else}
                  <span
                    class="overflow-hidden text-ellipsis whitespace-nowrap text-[var(--color-text-muted)]"
                  >
                    {op.reason}
                  </span>
                {/if}
                <span class="whitespace-nowrap text-right"
                  >{formatBytes(op.size_bytes)}</span
                >
              </div>
            {/snippet}
          </VirtualList>
        </div>
      {:else}
        <div class="p-3 border-t border-[var(--color-border-subtle)]">
          <VirtualGrid
            items={ops}
            itemKey={idOf}
            minTileWidth={SCALE_PX[$previewScale] + 56}
            tileHeight={SCALE_PX[$previewScale] + (isDedup ? 90 : 62)}
            gap={12}
            scrollElement={scrollRoot.el}
            layoutVersion={scrollRoot.layoutVersion}
          >
            {#snippet tile(op: Operation)}
              <div class="flex h-full flex-col items-center gap-1">
                {@render tileButton(op)}
                {#if isDedup}{@render keepButton(op)}{/if}
              </div>
            {/snippet}
          </VirtualGrid>
        </div>
      {/if}
    </Collapsible.Content>
  </Collapsible.Root>
{/each}
