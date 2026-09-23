<script lang="ts">
  import ApplyConfirmModal from "$lib/components/ApplyConfirmModal.svelte";
  import FolderPicker from "$lib/components/FolderPicker.svelte";
  import HistoryPanel from "$lib/components/HistoryPanel.svelte";
  import ModeSelector from "$lib/components/ModeSelector.svelte";
  import PreviewTable from "$lib/components/PreviewTable.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import { applyPlan, generatePlan } from "$lib/api/commands";
  import { formatBytes } from "$lib/format";
  import type { Plan, PlanRequest, RunRecord, ScanOptions } from "$lib/api/types";
  import { selectedRoot } from "$lib/state/stores";

  type Tab = "sort" | "settings" | "history";
  let tab = $state<Tab>("sort");

  let request = $state<PlanRequest>({ mode: "sort_by_type" });
  let scanOptions = $state<ScanOptions>({ include_subfolders: false, exclude: [] });
  let plan = $state<Plan | null>(null);
  let selected = $state<Record<string, boolean>>({});
  let scanning = $state(false);
  let applying = $state(false);
  let confirmOpen = $state(false);
  let errorMessage = $state<string | null>(null);
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
      errorMessage = "Choose a folder first.";
      return;
    }
    errorMessage = null;
    scanning = true;
    lastRun = null;
    try {
      plan = await generatePlan($selectedRoot, request, scanOptions);
      selected = Object.fromEntries(plan.operations.map((op) => [op.id, op.selected]));
    } catch (e) {
      errorMessage = `Scan failed: ${e}`;
      plan = null;
    } finally {
      scanning = false;
    }
  }

  async function confirmApply() {
    if (!plan) return;
    confirmOpen = false;
    applying = true;
    errorMessage = null;
    try {
      const ids = Object.entries(selected)
        .filter(([, v]) => v)
        .map(([id]) => id);
      lastRun = await applyPlan(plan, ids);
      showFailures = false;
      plan = null;
      selected = {};
    } catch (e) {
      errorMessage = `Apply failed: ${e}`;
    } finally {
      applying = false;
    }
  }
</script>

<main>
  <header>
    <h1>Sortty</h1>
    <nav>
      <button class:active={tab === "sort"} onclick={() => (tab = "sort")}>Sort &amp; Clean</button>
      <button class:active={tab === "history"} onclick={() => (tab = "history")}>History</button>
      <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>Settings</button>
    </nav>
  </header>

  {#if tab === "sort"}
    <section class="panel">
      <FolderPicker />
      <div class="spacer"></div>
      <ModeSelector bind:request />
      <div class="spacer"></div>
      <label class="subfolder-toggle">
        <input type="checkbox" bind:checked={scanOptions.include_subfolders} />
        Include files in subfolders too
      </label>
      {#if scanOptions.include_subfolders}
        <p class="warning">
          This will also reach into existing subfolders (installer folders, app folders, etc.) and move
          individual files out of them.
        </p>
      {/if}
      <div class="spacer"></div>
      <button type="button" class="primary" onclick={scan} disabled={scanning || !$selectedRoot}>
        {scanning ? "Scanning…" : "Scan"}
      </button>

      {#if errorMessage}<p class="error">{errorMessage}</p>{/if}

      {#if lastRun}
        <div class="result-banner">
          <p>
            Applied {lastRun.applied_operations.length} change(s){lastRun.failed_operations.length
              ? `, ${lastRun.failed_operations.length} failed`
              : ""}. You can undo this from the History tab.
          </p>
          {#if trashedBytes > 0}<p>{formatBytes(trashedBytes)} moved to trash (not yet permanently deleted).</p>{/if}
          {#if archivedBytes > 0}<p>{formatBytes(archivedBytes)} archived.</p>{/if}
          {#if lastRun.failed_operations.length > 0}
            <button type="button" class="ghost" onclick={() => (showFailures = !showFailures)}>
              {showFailures ? "Hide" : "Show"} failure details
            </button>
            {#if showFailures}
              <ul class="failure-list">
                {#each lastRun.failed_operations as f (f.operation.id)}
                  <li><code>{f.operation.source}</code> — {f.error}</li>
                {/each}
              </ul>
            {/if}
          {/if}
        </div>
      {/if}

      {#if plan}
        <div class="spacer"></div>
        <div class="preview-toolbar">
          <span>{selectedCount} of {plan.operations.length} selected ({formatBytes(selectedBytes)})</span>
          <button type="button" class="primary" onclick={() => (confirmOpen = true)} disabled={applying || selectedCount === 0}>
            {applying ? "Applying…" : `Apply ${selectedCount} change(s)`}
          </button>
        </div>
        <PreviewTable {plan} bind:selected />
      {/if}
    </section>
  {:else if tab === "history"}
    <section class="panel">
      <HistoryPanel />
    </section>
  {:else}
    <section class="panel">
      <SettingsPanel />
    </section>
  {/if}

  <ApplyConfirmModal bind:open={confirmOpen} {selectedCount} onConfirm={confirmApply} onCancel={() => (confirmOpen = false)} />
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
    --border-color: #d8d8d8;
    --card-bg: #ffffff;
    --input-bg: #ffffff;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #202020;
      --border-color: #444;
      --card-bg: #2b2b2b;
      --input-bg: #2b2b2b;
    }
  }

  main {
    max-width: 960px;
    margin: 0 auto;
    padding: 1.5rem;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
  }

  h1 {
    margin: 0;
    font-size: 1.4rem;
  }

  nav {
    display: flex;
    gap: 0.5rem;
  }

  nav button {
    padding: 0.4rem 0.9rem;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  nav button.active {
    background: #396cd8;
    border-color: #396cd8;
    color: white;
  }

  .panel {
    background: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 1.25rem;
  }

  .spacer {
    height: 1rem;
  }

  button.primary {
    padding: 0.55rem 1.2rem;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #396cd8;
    color: white;
    cursor: pointer;
  }

  button.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preview-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
  }

  .error {
    color: #c0392b;
    font-size: 0.85rem;
  }

  .result-banner {
    background: #e6f4ea;
    color: #1e6b34;
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
    font-size: 0.85rem;
    margin-top: 0.75rem;
  }

  .result-banner p {
    margin: 0.25rem 0;
  }

  .subfolder-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .warning {
    font-size: 0.8rem;
    color: #a15c00;
    margin: 0.35rem 0 0;
  }

  button.ghost {
    padding: 0.3rem 0.7rem;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: inherit;
    cursor: pointer;
    font-size: 0.8rem;
    margin-top: 0.4rem;
  }

  .failure-list {
    margin: 0.4rem 0 0;
    padding-left: 1.1rem;
    font-size: 0.8rem;
  }
</style>
