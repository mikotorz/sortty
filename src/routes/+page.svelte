<script lang="ts">
  import ApplyConfirmModal from "$lib/components/ApplyConfirmModal.svelte";
  import FolderPicker from "$lib/components/FolderPicker.svelte";
  import PreviewTable from "$lib/components/PreviewTable.svelte";
  import ModeSelector from "$lib/components/ModeSelector.svelte";
  import { applyPlan, generatePlan } from "$lib/api/commands";
  import { formatBytes } from "$lib/format";
  import { pushToast } from "$lib/state/toast";
  import type { Plan, PlanRequest, RunRecord, ScanOptions } from "$lib/api/types";
  import { selectedRoot } from "$lib/state/stores";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";

  let request = $state<PlanRequest>({ mode: "sort_by_type" });
  let scanOptions = $state<ScanOptions>({ include_subfolders: false, exclude: [] });
  let plan = $state<Plan | null>(null);
  let selected = $state<Record<string, boolean>>({});
  let scanning = $state(false);
  let applying = $state(false);
  let confirmOpen = $state(false);
  let lastRun = $state<RunRecord | null>(null);
  let showFailures = $state(false);

  let selectedCount = $derived(Object.values(selected).filter(Boolean).length);
  let selectedBytes = $derived(
    plan ? plan.operations.filter((op) => selected[op.id]).reduce((sum, op) => sum + op.size_bytes, 0) : 0,
  );
  let trashedBytes = $derived(
    lastRun
      ? lastRun.applied_operations.filter((o) => o.kind === "move_to_trash").reduce((s, o) => s + o.size_bytes, 0)
      : 0,
  );
  let archivedBytes = $derived(
    lastRun
      ? lastRun.applied_operations.filter((o) => o.kind === "archive").reduce((s, o) => s + o.size_bytes, 0)
      : 0,
  );

  async function scan() {
    if (!$selectedRoot) {
      pushToast("error", "Choose a folder first.");
      return;
    }
    scanning = true;
    lastRun = null;
    try {
      plan = await generatePlan($selectedRoot, request, scanOptions);
      selected = Object.fromEntries(plan.operations.map((op) => [op.id, op.selected]));
    } catch (e) {
      pushToast("error", `Scan failed: ${e}`);
      plan = null;
    } finally {
      scanning = false;
    }
  }

  async function confirmApply() {
    if (!plan) return;
    confirmOpen = false;
    applying = true;
    try {
      const ids = Object.entries(selected)
        .filter(([, v]) => v)
        .map(([id]) => id);
      lastRun = await applyPlan(plan, ids);
      showFailures = false;
      pushToast(
        "success",
        `Applied ${lastRun.applied_operations.length} change(s)${lastRun.failed_operations.length ? `, ${lastRun.failed_operations.length} failed` : ""}.`,
      );
      plan = null;
      selected = {};
    } catch (e) {
      pushToast("error", `Apply failed: ${e}`);
    } finally {
      applying = false;
    }
  }
</script>

<div class="flex flex-col gap-4 max-w-3xl">
  <div>
    <h1 class="text-lg font-semibold">Sort &amp; Clean</h1>
    <p class="text-sm text-[var(--color-text-muted)]">Pick a folder and a mode, review the plan, then apply.</p>
  </div>

  <div class="card flex flex-col gap-4">
    <FolderPicker />

    <ModeSelector bind:request />

    <label class="flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={scanOptions.include_subfolders} />
      Include files in subfolders too
    </label>
    {#if scanOptions.include_subfolders}
      <p class="text-xs -mt-2 rounded-md bg-[var(--color-warning-bg)] text-[var(--color-warning-fg)] px-2.5 py-1.5">
        This will also reach into existing subfolders (installer folders, app folders, etc.) and move
        individual files out of them.
      </p>
    {/if}

    <button
      type="button"
      class="btn-primary self-start px-4 py-2"
      onclick={scan}
      disabled={scanning || !$selectedRoot}
    >
      {#if scanning}<LoaderCircle size={15} class="animate-spin" />{/if}
      {scanning ? "Scanning…" : "Scan"}
    </button>

    {#if lastRun}
      <div class="rounded-md bg-[var(--color-success-bg)] text-[var(--color-success-fg)] px-3 py-2.5 text-sm flex flex-col gap-1">
        {#if trashedBytes > 0}<p class="m-0">{formatBytes(trashedBytes)} moved to trash (not yet permanently deleted).</p>{/if}
        {#if archivedBytes > 0}<p class="m-0">{formatBytes(archivedBytes)} archived.</p>{/if}
        {#if lastRun.failed_operations.length > 0}
          <button type="button" class="self-start underline text-xs" onclick={() => (showFailures = !showFailures)}>
            {showFailures ? "Hide" : "Show"} failure details
          </button>
          {#if showFailures}
            <ul class="text-xs pl-4 m-0">
              {#each lastRun.failed_operations as f (f.operation.id)}
                <li><code>{f.operation.source}</code> — {f.error}</li>
              {/each}
            </ul>
          {/if}
        {/if}
      </div>
    {/if}
  </div>

  {#if plan}
    <div class="card">
      <div class="flex items-center justify-between text-sm mb-3">
        <span class="text-[var(--color-text-muted)]">{selectedCount} of {plan.operations.length} selected ({formatBytes(selectedBytes)})</span>
        <button
          type="button"
          class="btn-primary"
          onclick={() => (confirmOpen = true)}
          disabled={applying || selectedCount === 0}
        >
          {#if applying}<LoaderCircle size={14} class="animate-spin" />{/if}
          {applying ? "Applying…" : `Apply ${selectedCount} change(s)`}
        </button>
      </div>
      <PreviewTable {plan} bind:selected />
    </div>
  {/if}
</div>

<ApplyConfirmModal bind:open={confirmOpen} {selectedCount} onConfirm={confirmApply} onCancel={() => (confirmOpen = false)} />
