<script lang="ts">
  import type { PlanRequest } from "../api/types";

  let { request = $bindable() }: { request: PlanRequest } = $props();

  const modes: { value: PlanRequest["mode"]; label: string; blurb: string }[] = [
    { value: "sort_by_type", label: "Sort by type", blurb: "Group files into Images/, Documents/, Videos/, etc." },
    { value: "sort_by_date", label: "Sort by date", blurb: "Group files into year/month folders." },
    { value: "dedup", label: "Find duplicates", blurb: "Find identical files and move extras to trash." },
    { value: "cleanup", label: "Clean up stale files", blurb: "Archive files untouched for a long time." },
  ];

  function setMode(mode: PlanRequest["mode"]) {
    switch (mode) {
      case "sort_by_type":
        request = { mode };
        break;
      case "sort_by_date":
        request = { mode, date_source: "modified", granularity: "year_month" };
        break;
      case "dedup":
        request = { mode, min_size_bytes: 1024, keep_strategy: "oldest_modified" };
        break;
      case "cleanup":
        request = { mode, stale_days: 180, date_source: "modified", action: "archive" };
        break;
    }
  }
</script>

<div class="mode-grid">
  {#each modes as m (m.value)}
    <button
      type="button"
      class="mode-card"
      class:active={request.mode === m.value}
      onclick={() => setMode(m.value)}
    >
      <strong>{m.label}</strong>
      <span>{m.blurb}</span>
    </button>
  {/each}
</div>

<div class="options">
  {#if request.mode === "sort_by_date"}
    <label>
      Date source
      <select bind:value={request.date_source}>
        <option value="modified">Last modified</option>
        <option value="created">Created</option>
      </select>
    </label>
    <label>
      Granularity
      <select bind:value={request.granularity}>
        <option value="year_month">Year / Month</option>
        <option value="year">Year only</option>
      </select>
    </label>
  {:else if request.mode === "dedup"}
    <label>
      Minimum file size to consider (bytes)
      <input type="number" min="0" bind:value={request.min_size_bytes} />
    </label>
    <label>
      Keep
      <select bind:value={request.keep_strategy}>
        <option value="oldest_modified">The oldest copy</option>
        <option value="shortest_path">The one with the shortest path</option>
      </select>
    </label>
  {:else if request.mode === "cleanup"}
    <label>
      Stale after (days)
      <input type="number" min="1" bind:value={request.stale_days} />
    </label>
    <label>
      Date source
      <select bind:value={request.date_source}>
        <option value="modified">Last modified</option>
        <option value="created">Created</option>
      </select>
    </label>
    <label>
      Action
      <select bind:value={request.action}>
        <option value="archive">Pre-select for archiving</option>
        <option value="flag_only">Only flag (I'll pick manually)</option>
      </select>
    </label>
  {/if}
</div>

<style>
  .mode-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.75rem;
  }
  .mode-card {
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--border-color, #ccc);
    background: var(--card-bg, #fff);
    color: inherit;
    cursor: pointer;
  }
  .mode-card.active {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px #396cd833;
  }
  .mode-card span {
    font-size: 0.8rem;
    opacity: 0.75;
  }
  .options {
    margin-top: 1rem;
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
  }
  .options label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  .options select,
  .options input {
    padding: 0.4rem 0.5rem;
    border-radius: 6px;
    border: 1px solid var(--border-color, #ccc);
  }
</style>
