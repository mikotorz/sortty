<script lang="ts">
  import BrowsePanel from "$lib/components/BrowsePanel.svelte";
</script>

<div class="flex flex-col gap-4 h-full min-h-0">
  <div>
    <h1 class="text-lg font-semibold">Browse</h1>
    <p class="text-sm text-[var(--color-text-muted)]">
      Look inside a sorted folder and delete files that don't belong.
    </p>
  </div>

  <div class="card flex flex-col min-h-80 flex-1">
    <BrowsePanel />
  </div>
</div>
