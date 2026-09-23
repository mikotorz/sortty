<script lang="ts">
  import HistoryPanel from "$lib/components/HistoryPanel.svelte";
</script>

<div class="flex flex-col gap-4 max-w-3xl">
  <div>
    <h1 class="text-lg font-semibold">History</h1>
    <p class="text-sm text-[var(--color-text-muted)]">
      Past runs, with undo for anything that hasn't been undone yet.
    </p>
  </div>

  <div class="card">
    <HistoryPanel />
  </div>
</div>
