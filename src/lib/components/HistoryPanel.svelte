<script lang="ts">
  import { onMount } from "svelte";
  import { getRun, listRuns, undoRun } from "../api/commands";
  import type { FailedOperation, RunSummary } from "../api/types";
  import { pushToast } from "../state/toast";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import History from "@lucide/svelte/icons/history";

  let runs = $state<RunSummary[]>([]);
  let loading = $state(true);
  let undoingId = $state<string | null>(null);
  let expandedFailures = $state<Record<string, FailedOperation[]>>({});
  let loadingFailuresFor = $state<string | null>(null);

  onMount(load);

  async function load() {
    loading = true;
    try {
      runs = await listRuns(50);
    } catch (e) {
      pushToast("error", `Couldn't load history: ${e}`);
    } finally {
      loading = false;
    }
  }

  const modeLabels: Record<string, string> = {
    sort_by_type: "Sort by type",
    sort_by_date: "Sort by date",
    dedup: "Find duplicates",
    cleanup: "Clean up stale files",
    delete: "Delete files",
  };

  async function handleUndo(runId: string) {
    undoingId = runId;
    try {
      const result = await undoRun(runId);
      pushToast(
        "success",
        result.conflicts.length > 0
          ? `Restored ${result.restored} file(s); ${result.conflicts.length} couldn't be restored (path is occupied).`
          : `Restored ${result.restored} file(s).`,
      );
      await load();
    } catch (e) {
      pushToast("error", `Undo failed: ${e}`);
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
      expandedFailures = {
        ...expandedFailures,
        [runId]: record.failed_operations,
      };
    } catch (e) {
      pushToast("error", `Couldn't load failure details: ${e}`);
    } finally {
      loadingFailuresFor = null;
    }
  }
</script>

{#if loading}
  <p class="flex items-center gap-2 text-sm text-[var(--color-text-muted)]">
    <LoaderCircle size={15} class="animate-spin" /> Loading history…
  </p>
{:else if runs.length === 0}
  <div
    class="flex flex-col items-center gap-2 py-10 text-[var(--color-text-muted)]"
  >
    <History size={28} />
    <p class="m-0 text-sm">
      No runs yet. Apply a plan from Sort &amp; Clean to see it here.
    </p>
  </div>
{:else}
  <table class="w-full border-collapse text-sm">
    <thead>
      <tr class="text-left text-xs text-[var(--color-text-muted)]">
        <th class="px-2.5 py-1.5 font-medium">When</th>
        <th class="px-2.5 py-1.5 font-medium">Mode</th>
        <th class="px-2.5 py-1.5 font-medium">Folder</th>
        <th class="px-2.5 py-1.5 font-medium">Applied</th>
        <th class="px-2.5 py-1.5 font-medium">Failed</th>
        <th class="px-2.5 py-1.5"></th>
      </tr>
    </thead>
    <tbody>
      {#each runs as run (run.run_id)}
        <tr class="hover:bg-[var(--color-surface-hover)]">
          <td class="border-t border-[var(--color-border-subtle)] px-2.5 py-1.5"
            >{new Date(run.started_at).toLocaleString()}</td
          >
          <td class="border-t border-[var(--color-border-subtle)] px-2.5 py-1.5"
            >{modeLabels[run.plan_mode] ?? run.plan_mode}</td
          >
          <td
            class="max-w-0 w-[35%] overflow-hidden text-ellipsis whitespace-nowrap border-t border-[var(--color-border-subtle)] px-2.5 py-1.5"
            title={run.root}
          >
            {run.root}
          </td>
          <td class="border-t border-[var(--color-border-subtle)] px-2.5 py-1.5"
            >{run.applied_count}</td
          >
          <td
            class="border-t border-[var(--color-border-subtle)] px-2.5 py-1.5"
          >
            <span class="inline-flex items-center gap-1">
              {run.failed_count}
              {#if run.failed_count > 0}
                <button
                  type="button"
                  class="inline-flex items-center gap-0.5 text-xs text-[var(--color-text-muted)] underline"
                  onclick={() => toggleFailures(run.run_id)}
                >
                  {loadingFailuresFor === run.run_id ? "…" : "why?"}
                  <ChevronDown
                    size={12}
                    class={expandedFailures[run.run_id] ? "rotate-180" : ""}
                  />
                </button>
              {/if}
            </span>
          </td>
          <td
            class="border-t border-[var(--color-border-subtle)] px-2.5 py-1.5"
          >
            {#if run.undone}
              <span class="text-xs italic text-[var(--color-text-muted)]"
                >Undone</span
              >
            {:else}
              <button
                type="button"
                class="btn-ghost px-2.5 py-1 text-xs"
                onclick={() => handleUndo(run.run_id)}
                disabled={undoingId === run.run_id}
              >
                {#if undoingId === run.run_id}<LoaderCircle
                    size={12}
                    class="animate-spin"
                  />{/if}
                {undoingId === run.run_id ? "Undoing…" : "Undo"}
              </button>
            {/if}
          </td>
        </tr>
        {#if expandedFailures[run.run_id]}
          <tr>
            <td colspan="6" class="px-2.5 pb-2">
              <ul class="m-0 pl-4 text-xs text-[var(--color-text-muted)]">
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
