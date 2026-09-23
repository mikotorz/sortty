<script lang="ts">
  import { Collapsible, Checkbox } from "bits-ui";
  import type { Operation, Plan } from "../api/types";
  import { formatBytes } from "../format";
  import { cn } from "../cn";
  import { previewViewMode, previewScale, SCALE_PX } from "../state/previewView";
  import FileThumb from "./FileThumb.svelte";
  import Search from "@lucide/svelte/icons/search";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Check from "@lucide/svelte/icons/check";
  import Minus from "@lucide/svelte/icons/minus";
  import FolderCheck from "@lucide/svelte/icons/folder-check";
  import List from "@lucide/svelte/icons/list";
  import LayoutGrid from "@lucide/svelte/icons/layout-grid";

  let { plan, selected = $bindable() }: { plan: Plan; selected: Record<string, boolean> } = $props();

  let query = $state("");
  let openGroups = $state<Record<string, boolean>>({});

  function dirOf(path: string): string {
    const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    return idx === -1 ? path : path.slice(0, idx);
  }

  function fileNameOf(path: string): string {
    const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    return idx === -1 ? path : path.slice(idx + 1);
  }

  let filteredOps = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return plan.operations;
    return plan.operations.filter(
      (op) => op.source.toLowerCase().includes(q) || op.destination.toLowerCase().includes(q),
    );
  });

  let groups = $derived.by(() => {
    const map = new Map<string, Operation[]>();
    for (const op of filteredOps) {
      const key = dirOf(op.destination);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(op);
    }
    return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  });

  function isGroupOpen(dir: string): boolean {
    return openGroups[dir] ?? true;
  }

  let allSelected = $derived(plan.operations.length > 0 && plan.operations.every((op) => selected[op.id]));
  let someSelected = $derived(!allSelected && plan.operations.some((op) => selected[op.id]));

  function toggleAll(value: boolean) {
    const next = { ...selected };
    for (const op of plan.operations) next[op.id] = value;
    selected = next;
  }

  function toggleGroup(ops: Operation[], value: boolean) {
    const next = { ...selected };
    for (const op of ops) next[op.id] = value;
    selected = next;
  }

  function groupChecked(ops: Operation[]): boolean {
    return ops.every((op) => selected[op.id]);
  }

  function groupIndeterminate(ops: Operation[]): boolean {
    return !groupChecked(ops) && ops.some((op) => selected[op.id]);
  }
</script>

<div class="flex items-center justify-between border-b border-[var(--color-border)] pb-2.5 mb-3 gap-3">
  <label class="flex items-center gap-2 text-sm font-medium">
    <Checkbox.Root
      checked={allSelected}
      indeterminate={someSelected}
      onCheckedChange={(v) => toggleAll(v === true)}
      class="chk"
    >
      {#snippet children({ checked, indeterminate })}
        {#if indeterminate}<Minus size={11} />{:else if checked}<Check size={11} />{/if}
      {/snippet}
    </Checkbox.Root>
    Select all ({plan.summary.total_files} files, {formatBytes(plan.summary.total_bytes)})
  </label>

  <div class="flex items-center gap-3">
    {#if $previewViewMode === "grid"}
      <div class="inline-flex rounded-md border border-[var(--color-border)] p-0.5 text-xs">
        {#each [["sm", "S"], ["md", "M"], ["lg", "L"]] as [value, label] ([value])}
          <button
            type="button"
            class={cn(
              "rounded px-2 py-1",
              $previewScale === value ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]" : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
            )}
            onclick={() => previewScale.set(value as "sm" | "md" | "lg")}
          >
            {label}
          </button>
        {/each}
      </div>
    {/if}
    <div class="inline-flex rounded-md border border-[var(--color-border)] p-0.5">
      <button
        type="button"
        aria-label="List view"
        class={cn(
          "flex items-center rounded p-1.5",
          $previewViewMode === "list" ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]" : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
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
          $previewViewMode === "grid" ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]" : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
        )}
        onclick={() => previewViewMode.set("grid")}
      >
        <LayoutGrid size={14} />
      </button>
    </div>
    <div class="relative">
      <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--color-text-muted)]" />
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
  <div class="flex flex-col items-center gap-2 py-10 text-[var(--color-text-muted)]">
    <FolderCheck size={28} />
    <p class="m-0 text-sm">Nothing to do — this folder already looks tidy for this mode.</p>
  </div>
{:else if groups.length === 0}
  <p class="py-6 text-sm text-[var(--color-text-muted)]">No operations match "{query}".</p>
{/if}

{#each groups as [dir, ops] (dir)}
  <Collapsible.Root
    class="group mb-2 rounded-md border border-[var(--color-border-subtle)] overflow-hidden"
    open={isGroupOpen(dir)}
    onOpenChange={(v) => (openGroups = { ...openGroups, [dir]: v })}
  >
    <div class="flex items-center gap-2 bg-[var(--color-surface-hover)] px-2.5 py-1.5">
      <Checkbox.Root
        checked={groupChecked(ops)}
        indeterminate={groupIndeterminate(ops)}
        onCheckedChange={(v) => toggleGroup(ops, v === true)}
        class="chk"
      >
        {#snippet children({ checked, indeterminate })}
          {#if indeterminate}<Minus size={11} />{:else if checked}<Check size={11} />{/if}
        {/snippet}
      </Checkbox.Root>
      <Collapsible.Trigger class="group-trigger flex flex-1 items-center gap-1.5 text-left text-sm">
        <ChevronRight size={14} class="chevron shrink-0 text-[var(--color-text-muted)]" />
        <code class="text-[0.8rem]">{dir}</code>
        <span class="text-xs text-[var(--color-text-muted)]">({ops.length})</span>
      </Collapsible.Trigger>
    </div>
    <Collapsible.Content>
      {#if $previewViewMode === "list"}
        <table class="w-full border-collapse text-sm">
          <tbody>
            {#each ops as op (op.id)}
              <tr class="hover:bg-[var(--color-surface-hover)]">
                <td class="w-8 px-2.5 py-1.5 border-t border-[var(--color-border-subtle)]">
                  <Checkbox.Root
                    checked={selected[op.id]}
                    onCheckedChange={(v) => (selected = { ...selected, [op.id]: v === true })}
                    class="chk"
                  >
                    {#snippet children({ checked })}
                      {#if checked}<Check size={11} />{/if}
                    {/snippet}
                  </Checkbox.Root>
                </td>
                <td class="max-w-0 w-[55%] overflow-hidden text-ellipsis whitespace-nowrap px-2.5 py-1.5 border-t border-[var(--color-border-subtle)]" title={op.source}>
                  {op.source}
                </td>
                <td class="px-2.5 py-1.5 border-t border-[var(--color-border-subtle)] text-[var(--color-text-muted)]">
                  {op.reason}
                </td>
                <td class="px-2.5 py-1.5 border-t border-[var(--color-border-subtle)] text-right whitespace-nowrap">
                  {formatBytes(op.size_bytes)}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <div
          class="grid gap-3 p-3 border-t border-[var(--color-border-subtle)]"
          style="grid-template-columns: repeat(auto-fill, minmax({SCALE_PX[$previewScale] + 56}px, 1fr));"
        >
          {#each ops as op (op.id)}
            <button
              type="button"
              class={cn(
                "relative flex flex-col items-center gap-1.5 rounded-lg border p-2.5 text-center",
                selected[op.id] ? "border-[var(--color-accent)] bg-[var(--color-surface-hover)]" : "border-transparent hover:bg-[var(--color-surface-hover)]",
              )}
              onclick={() => (selected = { ...selected, [op.id]: !selected[op.id] })}
            >
              <div class="chk absolute left-1.5 top-1.5" data-state={selected[op.id] ? "checked" : "unchecked"} aria-hidden="true">
                {#if selected[op.id]}<Check size={11} />{/if}
              </div>
              <FileThumb path={op.source} size={SCALE_PX[$previewScale]} />
              <span class="w-full truncate text-xs" title={op.source}>{fileNameOf(op.source)}</span>
              <span class="text-[0.7rem] text-[var(--color-text-muted)]">{formatBytes(op.size_bytes)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </Collapsible.Content>
  </Collapsible.Root>
{/each}

<style>
  :global(.chk) {
    width: 16px;
    height: 16px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    border: 1px solid var(--color-border);
    background: var(--color-bg);
    color: var(--color-accent-fg);
    padding: 0;
    cursor: pointer;
  }
  :global(.chk[data-state="checked"]),
  :global(.chk[data-state="indeterminate"]) {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
  :global(.chevron) {
    transition: transform 120ms ease;
  }
  :global(.group[data-state="open"] .chevron) {
    transform: rotate(90deg);
  }
</style>
