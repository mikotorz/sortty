<script lang="ts">
  import { onMount } from "svelte";
  import { getRun, listRuns, undoRun } from "../api/commands";
  import type { FailedOperation, RunSummary } from "../api/types";

  let runs = $state<RunSummary[]>([]);
  let loading = $state(true);
  let undoingId = $state<string | null>(null);
  let message = $state<string | null>(null);
  let expandedFailures = $state<Record<string, FailedOperation[]>>({});
  let loadingFailuresFor = $state<string | null>(null);

  onMount(load);

  async function load() {
    loading = true;
    runs = await listRuns(50);
    loading = false;
  }

  const modeLabels: Record<string, string> = {
    sort_by_type: "Sort by type",
    sort_by_date: "Sort by date",
    dedup: "Find duplicates",
    cleanup: "Clean up stale files",
  };

  async function handleUndo(runId: string) {
    undoingId = runId;
    message = null;
    try {
      const result = await undoRun(runId);
      message =
        result.conflicts.length > 0
          ? `Restored ${result.restored} file(s); ${result.conflicts.length} couldn't be restored (path is occupied).`
          : `Restored ${result.restored} file(s).`;
      await load();
    } catch (e) {
      message = `Undo failed: ${e}`;
    } finally {
      undoingId = null;
    }
  }

  async function toggleFailures(runId: string) {
    if (expandedFailures[runId]) {
      const { [runId]: _removed, ...rest } = expandedFailures;
      expandedFailures = rest;
      return;
    }
    loadingFailuresFor = runId;
    try {
      const record = await getRun(runId);
      expandedFailures = { ...expandedFailures, [runId]: record.failed_operations };
    } finally {
      loadingFailuresFor = null;
    }
  }
</script>

{#if loading}
  <p>Loading history…</p>
{:else if runs.length === 0}
  <p class="empty">No runs yet. Apply a plan from the Scan tab to see it here.</p>
{:else}
  {#if message}<p class="message">{message}</p>{/if}
  <table>
    <thead>
      <tr>
        <th>When</th>
        <th>Mode</th>
        <th>Folder</th>
        <th>Applied</th>
        <th>Failed</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each runs as run (run.run_id)}
        <tr>
          <td>{new Date(run.started_at).toLocaleString()}</td>
          <td>{modeLabels[run.plan_mode] ?? run.plan_mode}</td>
          <td class="path-cell" title={run.root}>{run.root}</td>
          <td>{run.applied_count}</td>
          <td>
            {run.failed_count}
            {#if run.failed_count > 0}
              <button type="button" class="link" onclick={() => toggleFailures(run.run_id)}>
                {loadingFailuresFor === run.run_id
                  ? "…"
                  : expandedFailures[run.run_id]
                    ? "hide"
                    : "why?"}
              </button>
            {/if}
          </td>
          <td>
            {#if run.undone}
              <span class="undone-badge">Undone</span>
            {:else}
              <button type="button" onclick={() => handleUndo(run.run_id)} disabled={undoingId === run.run_id}>
                {undoingId === run.run_id ? "Undoing…" : "Undo"}
              </button>
            {/if}
          </td>
        </tr>
        {#if expandedFailures[run.run_id]}
          <tr>
            <td colspan="6">
              <ul class="failure-list">
                {#each expandedFailures[run.run_id] as f (f.operation.id)}
                  <li><code>{f.operation.source}</code> — {f.error}</li>
                {/each}
              </ul>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .empty {
    opacity: 0.7;
  }
  .message {
    font-size: 0.85rem;
    padding: 0.5rem;
    background: var(--card-bg, #f0f0f0);
    border-radius: 6px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.4rem 0.5rem;
    border-bottom: 1px solid var(--border-color, #eee);
  }
  .path-cell {
    max-width: 0;
    width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .undone-badge {
    opacity: 0.6;
    font-style: italic;
  }
  button {
    padding: 0.3rem 0.7rem;
    border-radius: 6px;
    border: 1px solid var(--border-color, #ccc);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  button.link {
    padding: 0;
    border: none;
    background: none;
    text-decoration: underline;
    font-size: 0.8rem;
    opacity: 0.75;
  }
  .failure-list {
    margin: 0.3rem 0;
    padding-left: 1.1rem;
    font-size: 0.8rem;
    opacity: 0.85;
  }
</style>
