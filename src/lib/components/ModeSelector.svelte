<script lang="ts">
  import type { PlanRequest } from "../api/types";
  import { defaultRequestForMode } from "../state/planRequestDefaults";
  import { appSettings } from "../state/stores";
  import { cn } from "../cn";
  import FolderTree from "@lucide/svelte/icons/folder-tree";
  import Clock from "@lucide/svelte/icons/clock";
  import Fingerprint from "@lucide/svelte/icons/fingerprint";
  import Archive from "@lucide/svelte/icons/archive";

  let { request = $bindable() }: { request: PlanRequest } = $props();

  const modes: {
    value: PlanRequest["mode"];
    label: string;
    blurb: string;
    icon: typeof FolderTree;
  }[] = [
    {
      value: "sort_by_type",
      label: "Sort by type",
      blurb: "Group files into Images/, Documents/, Videos/, etc.",
      icon: FolderTree,
    },
    {
      value: "sort_by_date",
      label: "Sort by date",
      blurb: "Group files into year/month folders.",
      icon: Clock,
    },
    {
      value: "dedup",
      label: "Find duplicates",
      blurb: "Find identical files and move extras to trash.",
      icon: Fingerprint,
    },
    {
      value: "cleanup",
      label: "Clean up stale files",
      blurb: "Archive files untouched for a long time.",
      icon: Archive,
    },
  ];

  function setMode(mode: PlanRequest["mode"]) {
    request = defaultRequestForMode(mode, $appSettings);
  }
</script>

<div class="grid grid-cols-[repeat(auto-fit,minmax(180px,1fr))] gap-2.5">
  {#each modes as m (m.value)}
    <button
      type="button"
      class={cn(
        "flex flex-col items-start gap-1.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] p-3 text-left transition-colors",
        request.mode === m.value &&
          "border-[var(--color-accent)] bg-[var(--color-surface-hover)]",
      )}
      onclick={() => setMode(m.value)}
    >
      <m.icon
        size={17}
        class={request.mode === m.value
          ? "text-[var(--color-accent)]"
          : "text-[var(--color-text-muted)]"}
      />
      <strong class="text-sm font-medium">{m.label}</strong>
      <span class="text-xs text-[var(--color-text-muted)]">{m.blurb}</span>
    </button>
  {/each}
</div>

<div class="mt-3 flex flex-wrap gap-4">
  {#if request.mode === "sort_by_date"}
    <label class="flex flex-col gap-1 text-sm">
      Date source
      <select class="field" bind:value={request.date_source}>
        <option value="modified">Last modified</option>
        <option value="created">Created</option>
      </select>
    </label>
    <label class="flex flex-col gap-1 text-sm">
      Granularity
      <select class="field" bind:value={request.granularity}>
        <option value="year_month">Year / Month</option>
        <option value="year">Year only</option>
      </select>
    </label>
  {:else if request.mode === "dedup"}
    <label class="flex flex-col gap-1 text-sm">
      Minimum file size to consider (bytes)
      <input
        class="field"
        type="number"
        min="0"
        bind:value={request.min_size_bytes}
      />
    </label>
    <label class="flex flex-col gap-1 text-sm">
      Keep
      <select class="field" bind:value={request.keep_strategy}>
        <option value="oldest_modified">The oldest copy</option>
        <option value="shortest_path">The one with the shortest path</option>
      </select>
    </label>
  {:else if request.mode === "cleanup"}
    <label class="flex flex-col gap-1 text-sm">
      Stale after (days)
      <input
        class="field"
        type="number"
        min="1"
        bind:value={request.stale_days}
      />
    </label>
    <label class="flex flex-col gap-1 text-sm">
      Date source
      <select class="field" bind:value={request.date_source}>
        <option value="modified">Last modified</option>
        <option value="created">Created</option>
      </select>
    </label>
    <label class="flex flex-col gap-1 text-sm">
      Action
      <select class="field" bind:value={request.action}>
        <option value="archive">Pre-select for archiving</option>
        <option value="flag_only">Only flag (I'll pick manually)</option>
      </select>
    </label>
  {/if}
</div>
