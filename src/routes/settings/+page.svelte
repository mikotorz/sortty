<script lang="ts">
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
</script>

<div class="flex flex-col gap-4 max-w-3xl">
  <div>
    <h1 class="text-lg font-semibold">Settings</h1>
    <p class="text-sm text-[var(--color-text-muted)]">File type categories, cleanup thresholds, and trash/archive folder names.</p>
  </div>

  <div class="card">
    <SettingsPanel />
  </div>
</div>
