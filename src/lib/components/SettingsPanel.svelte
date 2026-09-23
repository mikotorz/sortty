<script lang="ts">
  import { onMount } from "svelte";
  import {
    getCategoryRules,
    getSettings,
    openConfigFolder,
    saveCategoryRules,
    saveSettings,
  } from "../api/commands";
  import type { AppSettings, CategoryRules } from "../api/types";
  import { pushToast } from "../state/toast";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import X from "@lucide/svelte/icons/x";

  let rules = $state<CategoryRules | null>(null);
  let appSettings = $state<AppSettings | null>(null);
  let extensionsText = $state<Record<string, string>>({});
  let newCategoryName = $state("");
  let loading = $state(true);
  let saving = $state(false);

  onMount(load);

  async function load() {
    loading = true;
    const [r, s] = await Promise.all([getCategoryRules(), getSettings()]);
    rules = r;
    appSettings = s;
    extensionsText = Object.fromEntries(
      Object.entries(r.categories).map(([name, def]) => [name, def.extensions.join(", ")]),
    );
    loading = false;
  }

  function addCategory() {
    if (!rules || !newCategoryName.trim()) return;
    rules.categories[newCategoryName.trim()] = { extensions: [] };
    extensionsText[newCategoryName.trim()] = "";
    newCategoryName = "";
  }

  function removeCategory(name: string) {
    if (!rules) return;
    delete rules.categories[name];
    delete extensionsText[name];
    rules = { ...rules };
  }

  async function saveAll() {
    if (!rules || !appSettings) return;
    saving = true;
    try {
      for (const [name, text] of Object.entries(extensionsText)) {
        rules.categories[name] = {
          extensions: text
            .split(",")
            .map((e) => e.trim().replace(/^\./, "").toLowerCase())
            .filter(Boolean),
        };
      }
      await Promise.all([saveCategoryRules(rules), saveSettings(appSettings)]);
      pushToast("success", "Settings saved.");
    } catch (e) {
      pushToast("error", `Save failed: ${e}`);
    } finally {
      saving = false;
    }
  }
</script>

{#if loading || !rules || !appSettings}
  <p class="flex items-center gap-2 text-sm text-[var(--color-text-muted)]">
    <LoaderCircle size={15} class="animate-spin" /> Loading settings…
  </p>
{:else}
  <section class="mb-6">
    <h3 class="mb-0.5 text-sm font-semibold">File type categories</h3>
    <p class="mt-0 mb-3 text-xs text-[var(--color-text-muted)]">Destination folder names and the extensions that route to them.</p>
    <div class="flex flex-col gap-2">
      {#each Object.keys(rules.categories) as name (name)}
        <div class="flex items-center gap-2">
          <span class="w-28 shrink-0 text-sm font-medium">{name}</span>
          <input class="field flex-1" type="text" bind:value={extensionsText[name]} placeholder="jpg, png, gif" />
          <button type="button" class="btn-ghost px-2 py-1.5" aria-label="Remove category" onclick={() => removeCategory(name)}>
            <X size={14} />
          </button>
        </div>
      {/each}
      <div class="flex items-center gap-2">
        <input class="field flex-1" type="text" placeholder="New category name" bind:value={newCategoryName} />
        <button type="button" class="btn-ghost" onclick={addCategory}>Add category</button>
      </div>
    </div>
    <label class="mt-3 flex items-center gap-2 text-sm">
      "Other" folder name
      <input class="field" type="text" bind:value={rules.other_folder_name} />
    </label>
  </section>

  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Stale-file cleanup</h3>
    <label class="flex items-center gap-2 text-sm">
      Stale after (days)
      <input class="field max-w-[8rem]" type="number" min="1" bind:value={appSettings.cleanup.stale_days} />
    </label>
  </section>

  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Duplicate detection</h3>
    <label class="flex items-center gap-2 text-sm">
      Minimum size (bytes)
      <input class="field max-w-[8rem]" type="number" min="0" bind:value={appSettings.dedup.min_size_bytes} />
    </label>
  </section>

  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Trash &amp; archive folders</h3>
    <div class="flex flex-col gap-2">
      <label class="flex items-center gap-2 text-sm">
        Trash folder name
        <input class="field max-w-[10rem]" type="text" bind:value={appSettings.trash.staging_folder_name} />
      </label>
      <label class="flex items-center gap-2 text-sm">
        Archive folder name
        <input class="field max-w-[10rem]" type="text" bind:value={appSettings.trash.archive_folder_name} />
      </label>
    </div>
  </section>

  <div class="flex items-center gap-3">
    <button type="button" class="btn-primary" onclick={saveAll} disabled={saving}>
      {#if saving}<LoaderCircle size={14} class="animate-spin" />{/if}
      {saving ? "Saving…" : "Save settings"}
    </button>
    <button type="button" class="btn-ghost" onclick={openConfigFolder}>Open config folder</button>
  </div>
{/if}
