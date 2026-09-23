<script lang="ts">
  import type { Operation, Plan } from "../api/types";
  import { formatBytes } from "../format";

  let { plan, selected = $bindable() }: { plan: Plan; selected: Record<string, boolean> } = $props();

  function dirOf(path: string): string {
    const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    return idx === -1 ? path : path.slice(0, idx);
  }

  let groups = $derived.by(() => {
    const map = new Map<string, Operation[]>();
    for (const op of plan.operations) {
      const key = dirOf(op.destination);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(op);
    }
    return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  });

  let allSelected = $derived(plan.operations.length > 0 && plan.operations.every((op) => selected[op.id]));

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
</script>

<div class="preview-header">
  <label>
    <input type="checkbox" checked={allSelected} onchange={(e) => toggleAll((e.target as HTMLInputElement).checked)} />
    Select all ({plan.summary.total_files} files, {formatBytes(plan.summary.total_bytes)})
  </label>
</div>

{#if plan.operations.length === 0}
  <p class="empty">Nothing to do — this folder already looks tidy for this mode.</p>
{/if}

{#each groups as [dir, ops] (dir)}
  <div class="group">
    <div class="group-header">
      <label>
        <input
          type="checkbox"
          checked={ops.every((op) => selected[op.id])}
          onchange={(e) => toggleGroup(ops, (e.target as HTMLInputElement).checked)}
        />
        <code>{dir}</code>
        <span class="count">({ops.length})</span>
      </label>
    </div>
    <table>
      <tbody>
        {#each ops as op (op.id)}
          <tr>
            <td class="checkbox-cell">
              <input type="checkbox" bind:checked={selected[op.id]} />
            </td>
            <td class="path-cell" title={op.source}>{op.source}</td>
            <td class="reason-cell">{op.reason}</td>
            <td class="size-cell">{formatBytes(op.size_bytes)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/each}

<style>
  .preview-header {
    padding: 0.5rem 0;
    font-weight: 600;
    border-bottom: 1px solid var(--border-color, #ccc);
    margin-bottom: 0.5rem;
  }
  .empty {
    opacity: 0.7;
    padding: 1rem 0;
  }
  .group {
    margin-bottom: 1rem;
  }
  .group-header {
    font-size: 0.85rem;
    padding: 0.35rem 0;
  }
  .group-header .count {
    opacity: 0.6;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  td {
    padding: 0.3rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #eee);
  }
  .checkbox-cell {
    width: 2rem;
  }
  .path-cell {
    max-width: 0;
    width: 55%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .reason-cell {
    opacity: 0.75;
  }
  .size-cell {
    text-align: right;
    white-space: nowrap;
  }
</style>
