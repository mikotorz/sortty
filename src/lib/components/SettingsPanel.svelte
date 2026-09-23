<script lang="ts">
  import { onMount } from "svelte";
  import { AlertDialog } from "bits-ui";
  import { fade, fly } from "svelte/transition";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import {
    getCategoryRules,
    getSettings,
    openConfigFolder,
    resetCategoryRules,
    resetSettings,
    saveCategoryRules,
    saveSettings,
  } from "../api/commands";
  import type { AppSettings, CategoryRules } from "../api/types";
  import { pushToast } from "../state/toast";
  import { theme, type Theme } from "../state/theme";
  import { cn } from "../cn";
  import ConfirmModal from "./ConfirmModal.svelte";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import X from "@lucide/svelte/icons/x";
  import Sun from "@lucide/svelte/icons/sun";
  import Moon from "@lucide/svelte/icons/moon";
  import Monitor from "@lucide/svelte/icons/monitor";

  let rules = $state<CategoryRules | null>(null);
  let appSettings = $state<AppSettings | null>(null);
  let extensionsText = $state<Record<string, string>>({});
  let newCategoryName = $state("");
  let loading = $state(true);
  let saving = $state(false);
  let resetting = $state(false);
  let confirmResetOpen = $state(false);

  let checkingUpdate = $state(false);
  let updateAvailable = $state<Update | null>(null);
  let updateConfirmOpen = $state(false);
  let installingUpdate = $state(false);

  onMount(load);

  async function load() {
    loading = true;
    try {
      const [r, s] = await Promise.all([getCategoryRules(), getSettings()]);
      rules = r;
      appSettings = s;
      extensionsText = Object.fromEntries(
        Object.entries(r.categories).map(([name, def]) => [
          name,
          def.extensions.join(", "),
        ]),
      );
    } catch (e) {
      pushToast("error", `Couldn't load settings: ${e}`);
    } finally {
      loading = false;
    }
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

  async function restoreDefaults() {
    confirmResetOpen = false;
    resetting = true;
    try {
      const [r, s] = await Promise.all([resetCategoryRules(), resetSettings()]);
      rules = r;
      appSettings = s;
      extensionsText = Object.fromEntries(
        Object.entries(r.categories).map(([name, def]) => [
          name,
          def.extensions.join(", "),
        ]),
      );
      pushToast("success", "Settings restored to defaults.");
    } catch (e) {
      pushToast("error", `Restore failed: ${e}`);
    } finally {
      resetting = false;
    }
  }

  async function checkForUpdates() {
    checkingUpdate = true;
    try {
      const update = await check();
      if (!update) {
        pushToast("success", "You're up to date.");
        return;
      }
      updateAvailable = update;
      updateConfirmOpen = true;
    } catch (e) {
      pushToast("error", `Couldn't check for updates: ${e}`);
    } finally {
      checkingUpdate = false;
    }
  }

  async function confirmInstallUpdate() {
    if (!updateAvailable) return;
    updateConfirmOpen = false;
    installingUpdate = true;
    try {
      await updateAvailable.downloadAndInstall();
      await relaunch();
    } catch (e) {
      pushToast("error", `Update failed: ${e}`);
      installingUpdate = false;
    }
  }

  const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: "system", label: "System", icon: Monitor },
    { value: "light", label: "Light", icon: Sun },
    { value: "dark", label: "Dark", icon: Moon },
  ];
</script>

{#if loading || !rules || !appSettings}
  <p class="flex items-center gap-2 text-sm text-[var(--color-text-muted)]">
    <LoaderCircle size={15} class="animate-spin" /> Loading settings…
  </p>
{:else}
  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Appearance</h3>
    <div
      class="inline-flex rounded-md border border-[var(--color-border)] p-0.5"
    >
      {#each themeOptions as opt (opt.value)}
        <button
          type="button"
          class={cn(
            "flex items-center gap-1.5 rounded px-3 py-1.5 text-sm",
            $theme === opt.value
              ? "bg-[var(--color-accent)] text-[var(--color-accent-fg)]"
              : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-hover)]",
          )}
          onclick={() => theme.set(opt.value)}
        >
          <opt.icon size={14} />
          {opt.label}
        </button>
      {/each}
    </div>
  </section>

  <section class="mb-6">
    <h3 class="mb-0.5 text-sm font-semibold">File type categories</h3>
    <p class="mt-0 mb-3 text-xs text-[var(--color-text-muted)]">
      Destination folder names and the extensions that route to them.
    </p>
    <div class="flex flex-col gap-2">
      {#each Object.keys(rules.categories) as name (name)}
        <div class="flex items-center gap-2">
          <span class="w-28 shrink-0 text-sm font-medium">{name}</span>
          <input
            class="field flex-1"
            type="text"
            bind:value={extensionsText[name]}
            placeholder="jpg, png, gif"
          />
          <button
            type="button"
            class="btn-ghost px-2 py-1.5"
            aria-label="Remove category"
            onclick={() => removeCategory(name)}
          >
            <X size={14} />
          </button>
        </div>
      {/each}
      <div class="flex items-center gap-2">
        <input
          class="field flex-1"
          type="text"
          placeholder="New category name"
          bind:value={newCategoryName}
        />
        <button type="button" class="btn-ghost" onclick={addCategory}
          >Add category</button
        >
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
      <input
        class="field max-w-[8rem]"
        type="number"
        min="1"
        bind:value={appSettings.cleanup.stale_days}
      />
    </label>
  </section>

  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Duplicate detection</h3>
    <label class="flex items-center gap-2 text-sm">
      Minimum size (bytes)
      <input
        class="field max-w-[8rem]"
        type="number"
        min="0"
        bind:value={appSettings.dedup.min_size_bytes}
      />
    </label>
  </section>

  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Trash &amp; archive folders</h3>
    <div class="flex flex-col gap-2">
      <label class="flex items-center gap-2 text-sm">
        Trash folder name
        <input
          class="field max-w-[10rem]"
          type="text"
          bind:value={appSettings.trash.staging_folder_name}
        />
      </label>
      <label class="flex items-center gap-2 text-sm">
        Archive folder name
        <input
          class="field max-w-[10rem]"
          type="text"
          bind:value={appSettings.trash.archive_folder_name}
        />
      </label>
    </div>
  </section>

  <section class="mb-6">
    <h3 class="mb-2 text-sm font-semibold">Updates</h3>
    <button
      type="button"
      class="btn-ghost"
      onclick={checkForUpdates}
      disabled={checkingUpdate}
    >
      {#if checkingUpdate}<LoaderCircle size={14} class="animate-spin" />{/if}
      {checkingUpdate ? "Checking…" : "Check for updates"}
    </button>
  </section>

  <div class="flex items-center gap-3">
    <button
      type="button"
      class="btn-primary"
      onclick={saveAll}
      disabled={saving}
    >
      {#if saving}<LoaderCircle size={14} class="animate-spin" />{/if}
      {saving ? "Saving…" : "Save settings"}
    </button>
    <button type="button" class="btn-ghost" onclick={openConfigFolder}
      >Open config folder</button
    >
    <button
      type="button"
      class="btn-ghost"
      onclick={() => (confirmResetOpen = true)}
      disabled={resetting}
    >
      {#if resetting}<LoaderCircle
          size={14}
          class="animate-spin"
        />{:else}<RotateCcw size={14} />{/if}
      Restore defaults
    </button>
  </div>

  <AlertDialog.Root bind:open={confirmResetOpen}>
    <AlertDialog.Portal>
      <AlertDialog.Overlay class="fixed inset-0 z-40 bg-black/40" forceMount>
        {#snippet child({ props, open })}
          {#if open}
            <div {...props} transition:fade={{ duration: 120 }}></div>
          {/if}
        {/snippet}
      </AlertDialog.Overlay>
      <AlertDialog.Content
        class="fixed left-1/2 top-1/2 z-50 w-[90%] max-w-[380px] -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-card)] bg-[var(--color-surface)] p-6 shadow-2xl outline-none"
        forceMount
      >
        {#snippet child({ props, open })}
          {#if open}
            <div {...props} transition:fly={{ y: 8, duration: 140 }}>
              <AlertDialog.Title class="text-base font-semibold m-0"
                >Restore default settings?</AlertDialog.Title
              >
              <AlertDialog.Description
                class="mt-2 text-sm leading-relaxed text-[var(--color-text-muted)]"
              >
                This replaces your file type categories, cleanup thresholds, and
                trash/archive folder names with the built-in defaults. This
                can't be undone.
              </AlertDialog.Description>
              <div class="mt-4 flex justify-end gap-2">
                <AlertDialog.Cancel class="btn-ghost">Cancel</AlertDialog.Cancel
                >
                <AlertDialog.Action
                  class="btn-primary"
                  onclick={restoreDefaults}>Restore defaults</AlertDialog.Action
                >
              </div>
            </div>
          {/if}
        {/snippet}
      </AlertDialog.Content>
    </AlertDialog.Portal>
  </AlertDialog.Root>

  <ConfirmModal
    bind:open={updateConfirmOpen}
    title="Update to v{updateAvailable?.version}?"
    confirmLabel={installingUpdate ? "Installing…" : "Download and install"}
    disabled={installingUpdate}
    onConfirm={confirmInstallUpdate}
    onCancel={() => (updateConfirmOpen = false)}
  >
    {#snippet description()}
      Sortty will download and install version {updateAvailable?.version}, then
      restart.
    {/snippet}
  </ConfirmModal>
{/if}
