<script lang="ts">
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import type { AppliedOperation, RunRecord } from "../api/types";
  import { dirOf, fileNameOf } from "../grouping";
  import { pushToast } from "../state/toast";
  import { getScrollRoot } from "../state/scrollRoot.svelte";
  import VirtualList from "./VirtualList.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import ArrowRight from "@lucide/svelte/icons/arrow-right";

  /**
   * What one run actually did: every file it moved (from → to), marked
   * where an undo has already put it back, plus anything that failed. The
   * moved list is virtualized, since a single run can move thousands of
   * files.
   */
  let { record }: { record: RunRecord } = $props();

  const scrollRoot = getScrollRoot();

  let restored = $derived(new Set(record.restored_ids));

  /** Where the file is now: back at `from` if restored, otherwise `to`. */
  function currentPath(op: AppliedOperation): string {
    return restored.has(op.id) || record.undone ? op.from : op.to;
  }

  async function reveal(path: string) {
    try {
      await revealItemInDir(path);
    } catch (e) {
      pushToast("error", `Couldn't show that file: ${e}`);
    }
  }
</script>

<div class="flex flex-col gap-3 py-2">
  {#if record.applied_operations.length > 0}
    <div class="text-xs" role="table" aria-label="Files moved in this run">
      <VirtualList
        items={record.applied_operations}
        estimateSize={30}
        scrollElement={scrollRoot.el}
        layoutVersion={scrollRoot.layoutVersion}
      >
        {#snippet row(op: AppliedOperation)}
          <div
            role="row"
            class="grid h-full items-center gap-2 border-t border-[var(--color-border-subtle)] px-1"
            style="grid-template-columns: minmax(0, 1fr) 1rem minmax(0, 1fr) auto auto;"
          >
            <span class="truncate" title={op.from}>{fileNameOf(op.from)}</span>
            <ArrowRight size={12} class="text-[var(--color-text-muted)]" />
            <span class="truncate text-[var(--color-text-muted)]" title={op.to}
              >{dirOf(op.to)}</span
            >
            <span
              class="whitespace-nowrap text-[var(--color-text-muted)] italic"
              >{restored.has(op.id) || record.undone ? "Restored" : ""}</span
            >
            <button
              type="button"
              class="btn-ghost px-1.5 py-0.5"
              aria-label="Show in folder"
              title="Show in folder"
              onclick={() => reveal(currentPath(op))}
            >
              <FolderOpen size={12} />
            </button>
          </div>
        {/snippet}
      </VirtualList>
    </div>
  {/if}

  {#if record.failed_operations.length > 0}
    <div>
      <p class="m-0 mb-1 text-xs font-medium">
        Couldn't move {record.failed_operations.length} file(s):
      </p>
      <ul class="m-0 pl-4 text-xs text-[var(--color-text-muted)]">
        {#each record.failed_operations as f (f.operation.id)}
          <li><code>{f.operation.source}</code> — {f.error}</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
