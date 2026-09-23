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

  let rules = $state<CategoryRules | null>(null);
  let appSettings = $state<AppSettings | null>(null);
  let extensionsText = $state<Record<string, string>>({});
  let newCategoryName = $state("");
  let saveMessage = $state<string | null>(null);
  let loading = $state(true);

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
    for (const [name, text] of Object.entries(extensionsText)) {
      rules.categories[name] = {
        extensions: text
          .split(",")
          .map((e) => e.trim().replace(/^\./, "").toLowerCase())
          .filter(Boolean),
      };
    }
    await Promise.all([saveCategoryRules(rules), saveSettings(appSettings)]);
    saveMessage = "Saved.";
    setTimeout(() => (saveMessage = null), 2000);
  }
</script>

{#if loading || !rules || !appSettings}
  <p>Loading settings…</p>
{:else}
  <section>
    <h3>File type categories</h3>
    <p class="hint">Destination folder names and the extensions that route to them.</p>
    {#each Object.keys(rules.categories) as name (name)}
      <div class="category-row">
        <span class="category-name">{name}</span>
        <input type="text" bind:value={extensionsText[name]} placeholder="jpg, png, gif" />
        <button type="button" class="ghost" onclick={() => removeCategory(name)}>Remove</button>
      </div>
    {/each}
    <div class="category-row">
      <input type="text" placeholder="New category name" bind:value={newCategoryName} />
      <button type="button" onclick={addCategory}>Add category</button>
    </div>
    <label class="inline">
      "Other" folder name
      <input type="text" bind:value={rules.other_folder_name} />
    </label>
  </section>

  <section>
    <h3>Stale-file cleanup</h3>
    <label class="inline">
      Stale after (days)
      <input type="number" min="1" bind:value={appSettings.cleanup.stale_days} />
    </label>
  </section>

  <section>
    <h3>Duplicate detection</h3>
    <label class="inline">
      Minimum size (bytes)
      <input type="number" min="0" bind:value={appSettings.dedup.min_size_bytes} />
    </label>
  </section>

  <section>
    <h3>Trash &amp; archive folders</h3>
    <label class="inline">
      Trash folder name
      <input type="text" bind:value={appSettings.trash.staging_folder_name} />
    </label>
    <label class="inline">
      Archive folder name
      <input type="text" bind:value={appSettings.trash.archive_folder_name} />
    </label>
  </section>

  <div class="footer">
    <button type="button" onclick={saveAll}>Save settings</button>
    <button type="button" class="ghost" onclick={openConfigFolder}>Open config folder</button>
    {#if saveMessage}<span class="save-message">{saveMessage}</span>{/if}
  </div>
{/if}

<style>
  section {
    margin-bottom: 1.5rem;
  }
  h3 {
    margin-bottom: 0.25rem;
  }
  .hint {
    font-size: 0.8rem;
    opacity: 0.7;
    margin-top: 0;
  }
  .category-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.4rem;
  }
  .category-name {
    width: 120px;
    font-weight: 600;
  }
  input[type="text"],
  input[type="number"] {
    padding: 0.4rem 0.5rem;
    border-radius: 6px;
    border: 1px solid var(--border-color, #ccc);
    flex: 1;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    margin-bottom: 0.4rem;
  }
  .inline input {
    max-width: 160px;
  }
  button {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #396cd8;
    color: white;
    cursor: pointer;
  }
  button.ghost {
    background: transparent;
    border-color: var(--border-color, #ccc);
    color: inherit;
  }
  .footer {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .save-message {
    font-size: 0.85rem;
    opacity: 0.7;
  }
</style>
