<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import ConfirmModal from "$lib/components/ConfirmModal.svelte";
  import FolderPicker from "$lib/components/FolderPicker.svelte";
  import PreviewTable from "$lib/components/PreviewTable.svelte";
  import ModeSelector from "$lib/components/ModeSelector.svelte";
  import {
    applyPlan,
    cancelCurrentOperation,
    generatePlan,
  } from "$lib/api/commands";
  import { formatBytes } from "$lib/format";
  import { pushToast } from "$lib/state/toast";
  import { selectedRoot } from "$lib/state/stores";
  import { scanSession } from "$lib/state/scanSession.svelte";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import X from "@lucide/svelte/icons/x";

  async function addExcludedFolder() {
    const path = await open({
      directory: true,
      multiple: false,
      title: "Choose a subfolder to exclude",
    });
    if (
      typeof path === "string" &&
      !scanSession.scanOptions.exclude_folders.includes(path)
    ) {
      scanSession.scanOptions.exclude_folders = [
        ...scanSession.scanOptions.exclude_folders,
        path,
      ];
    }
  }

  function removeExcludedFolder(path: string) {
    scanSession.scanOptions.exclude_folders =
      scanSession.scanOptions.exclude_folders.filter((p) => p !== path);
  }

  let confirmOpen = $state(false);

  let selectedCount = $derived(
    Object.values(scanSession.selected).filter(Boolean).length,
  );
  let selectedBytes = $derived(
    scanSession.plan
      ? scanSession.plan.operations
          .filter((op) => scanSession.selected[op.id])
          .reduce((sum, op) => sum + op.size_bytes, 0)
      : 0,
  );
  let trashedBytes = $derived(
    scanSession.lastRun
      ? scanSession.lastRun.applied_operations
          .filter((o) => o.kind === "move_to_trash")
          .reduce((s, o) => s + o.size_bytes, 0)
      : 0,
  );
  let archivedBytes = $derived(
    scanSession.lastRun
      ? scanSession.lastRun.applied_operations
          .filter((o) => o.kind === "archive")
          .reduce((s, o) => s + o.size_bytes, 0)
      : 0,
  );

  async function scan() {
    if (!$selectedRoot) {
      pushToast("error", "Choose a folder first.");
      return;
    }
    scanSession.scanning = true;
    scanSession.scanProgress = 0;
    scanSession.lastRun = null;
    try {
      const plan = await generatePlan(
        $selectedRoot,
        scanSession.request,
        scanSession.scanOptions,
        (count) => (scanSession.scanProgress = count),
      );
      if (plan === null) {
        pushToast("info", "Scan cancelled.");
        scanSession.plan = null;
        return;
      }
      scanSession.plan = plan;
      scanSession.selected = Object.fromEntries(
        plan.operations.map((op) => [op.id, op.selected]),
      );
    } catch (e) {
      pushToast("error", `Scan failed: ${e}`);
      scanSession.plan = null;
    } finally {
      scanSession.scanning = false;
      scanSession.scanProgress = null;
    }
  }

  async function confirmApply() {
    if (!scanSession.plan) return;
    confirmOpen = false;
    scanSession.applying = true;
    scanSession.applyProgress = null;
    try {
      const ids = Object.entries(scanSession.selected)
        .filter(([, v]) => v)
        .map(([id]) => id);
      const run = await applyPlan(
        scanSession.plan,
        ids,
        (completed, total) =>
          (scanSession.applyProgress = { completed, total }),
      );
      scanSession.lastRun = run;
      scanSession.showFailures = false;
      if (run.cancelled) {
        pushToast(
          "info",
          `Cancelled after ${run.applied_operations.length} change(s).`,
        );
      } else {
        pushToast(
          "success",
          `Applied ${run.applied_operations.length} change(s)${run.failed_operations.length ? `, ${run.failed_operations.length} failed` : ""}.`,
        );
      }
      scanSession.plan = null;
      scanSession.selected = {};
    } catch (e) {
      pushToast("error", `Apply failed: ${e}`);
    } finally {
      scanSession.applying = false;
      scanSession.applyProgress = null;
    }
  }
</script>

<div class="flex flex-col gap-4 max-w-3xl h-full min-h-0">
  <div>
    <h1 class="text-lg font-semibold">Sort &amp; Clean</h1>
    <p class="text-sm text-[var(--color-text-muted)]">
      Pick a folder and a mode, review the plan, then apply.
    </p>
  </div>

  <div class="card flex flex-col gap-4">
    <FolderPicker />

    <ModeSelector bind:request={scanSession.request} />

    <label class="flex items-center gap-2 text-sm">
      <input
        type="checkbox"
        bind:checked={scanSession.scanOptions.include_subfolders}
      />
      Include files in subfolders too
    </label>
    {#if scanSession.scanOptions.include_subfolders}
      <p
        class="text-xs -mt-2 rounded-md bg-[var(--color-warning-bg)] text-[var(--color-warning-fg)] px-2.5 py-1.5"
      >
        This will also reach into existing subfolders (installer folders, app
        folders, etc.) and move individual files out of them.
      </p>

      <div class="flex flex-col gap-2">
        <span class="text-sm font-medium">Exclude specific subfolders</span>
        {#each scanSession.scanOptions.exclude_folders as path (path)}
          <div class="flex items-center gap-2">
            <span class="field flex-1 truncate" title={path}>{path}</span>
            <button
              type="button"
              class="btn-ghost px-2 py-1.5"
              aria-label="Stop excluding this folder"
              onclick={() => removeExcludedFolder(path)}
            >
              <X size={14} />
            </button>
          </div>
        {/each}
        <button
          type="button"
          class="btn-ghost self-start"
          onclick={addExcludedFolder}>Exclude a folder…</button
        >
      </div>
    {/if}

    <div class="flex items-center gap-2">
      <button
        type="button"
        class="btn-primary px-4 py-2"
        onclick={scan}
        disabled={scanSession.scanning || !$selectedRoot}
      >
        {#if scanSession.scanning}<LoaderCircle
            size={15}
            class="animate-spin"
          />{/if}
        {scanSession.scanning
          ? `Scanning… (${scanSession.scanProgress ?? 0} found)`
          : "Scan"}
      </button>
      {#if scanSession.scanning}
        <button
          type="button"
          class="btn-ghost px-3 py-2"
          onclick={cancelCurrentOperation}
        >
          Cancel
        </button>
      {/if}
    </div>

    {#if scanSession.lastRun}
      <div
        class="rounded-md bg-[var(--color-success-bg)] text-[var(--color-success-fg)] px-3 py-2.5 text-sm flex flex-col gap-1"
      >
        {#if trashedBytes > 0}<p class="m-0">
            {formatBytes(trashedBytes)} moved to trash (not yet permanently deleted).
          </p>{/if}
        {#if archivedBytes > 0}<p class="m-0">
            {formatBytes(archivedBytes)} archived.
          </p>{/if}
        {#if scanSession.lastRun.failed_operations.length > 0}
          <button
            type="button"
            class="self-start underline text-xs"
            onclick={() =>
              (scanSession.showFailures = !scanSession.showFailures)}
          >
            {scanSession.showFailures ? "Hide" : "Show"} failure details
          </button>
          {#if scanSession.showFailures}
            <ul class="text-xs pl-4 m-0">
              {#each scanSession.lastRun.failed_operations as f (f.operation.id)}
                <li><code>{f.operation.source}</code> — {f.error}</li>
              {/each}
            </ul>
          {/if}
        {/if}
      </div>
    {/if}
  </div>

  {#if scanSession.plan}
    <div class="card flex flex-col min-h-80 flex-1">
      <div class="flex items-center justify-between text-sm mb-3">
        <span class="text-[var(--color-text-muted)]"
          >{selectedCount} of {scanSession.plan.operations.length} selected ({formatBytes(
            selectedBytes,
          )})</span
        >
        <div class="flex items-center gap-2">
          {#if scanSession.applying && scanSession.applyProgress}
            <div
              class="h-1.5 w-24 overflow-hidden rounded-full bg-[var(--color-border)]"
            >
              <div
                class="h-full bg-[var(--color-accent)]"
                style="width: {(scanSession.applyProgress.completed /
                  scanSession.applyProgress.total) *
                  100}%"
              ></div>
            </div>
          {/if}
          <button
            type="button"
            class="btn-primary"
            onclick={() => (confirmOpen = true)}
            disabled={scanSession.applying || selectedCount === 0}
          >
            {#if scanSession.applying}<LoaderCircle
                size={14}
                class="animate-spin"
              />{/if}
            {#if scanSession.applying}
              {scanSession.applyProgress
                ? `Applying… (${scanSession.applyProgress.completed}/${scanSession.applyProgress.total})`
                : "Applying…"}
            {:else}
              Apply {selectedCount} change(s)
            {/if}
          </button>
          {#if scanSession.applying}
            <button
              type="button"
              class="btn-ghost px-3 py-1.5 text-sm"
              onclick={cancelCurrentOperation}
            >
              Cancel
            </button>
          {/if}
        </div>
      </div>
      <PreviewTable
        plan={scanSession.plan}
        bind:selected={scanSession.selected}
      />
    </div>
  {/if}
</div>

<ConfirmModal
  bind:open={confirmOpen}
  title="Apply {selectedCount} change{selectedCount === 1 ? '' : 's'}?"
  confirmLabel="Apply {selectedCount} change{selectedCount === 1 ? '' : 's'}"
  disabled={selectedCount === 0}
  onConfirm={confirmApply}
  onCancel={() => (confirmOpen = false)}
>
  {#snippet description()}
    Files will be moved to their new locations now. Nothing is permanently
    deleted — duplicates and stale files go into a <code>.sortty-trash</code> /
    <code>.sortty-archive</code> folder inside the scanned folder (not the Windows
    Recycle Bin), and this whole run can be undone from History afterward.
  {/snippet}
</ConfirmModal>
