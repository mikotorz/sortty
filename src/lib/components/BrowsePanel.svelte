<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { Collapsible, Checkbox } from "bits-ui";
  import { browseFolder, deleteFiles } from "../api/commands";
  import type { FileEntry } from "../api/types";
  import { formatBytes } from "../format";
  import { pushToast } from "../state/toast";
  import DeleteConfirmModal from "./DeleteConfirmModal.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Check from "@lucide/svelte/icons/check";
  import Minus from "@lucide/svelte/icons/minus";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import FolderSearch from "@lucide/svelte/icons/folder-search";

  let folder = $state<string | null>(null);
  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let deleting = $state(false);
  let confirmOpen = $state(false);
  let selected = $state<Record<string, boolean>>({});
  let openGroups = $state<Record<string, boolean>>({});

  function dirOf(path: string): string {
    const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    return idx === -1 ? path : path.slice(0, idx);
  }

  let groups = $derived.by(() => {
    const map = new Map<string, FileEntry[]>();
    for (const entry of entries) {
      const key = dirOf(entry.path);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(entry);
    }
    return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  });

  let selectedCount = $derived(Object.values(selected).filter(Boolean).length);

  function isGroupOpen(dir: string): boolean {
    return openGroups[dir] ?? true;
  }

  function groupChecked(group: FileEntry[]): boolean {
    return group.every((e) => selected[e.path]);
  }

  function groupIndeterminate(group: FileEntry[]): boolean {
    return !groupChecked(group) && group.some((e) => selected[e.path]);
  }

  function toggleGroup(group: FileEntry[], value: boolean) {
    const next = { ...selected };
    for (const e of group) next[e.path] = value;
    selected = next;
  }

  async function pickFolder() {
    const path = await open({ directory: true, multiple: false, title: "Choose a sorted folder to browse" });
    if (typeof path === "string") {
      folder = path;
      await reload();
    }
  }

  async function reload() {
    if (!folder) return;
    loading = true;
    selected = {};
    try {
      entries = await browseFolder(folder);
    } catch (e) {
      pushToast("error", `Couldn't read that folder: ${e}`);
      entries = [];
    } finally {
      loading = false;
    }
  }

  async function confirmDelete() {
    if (!folder) return;
    confirmOpen = false;
    deleting = true;
    try {
      const paths = Object.entries(selected)
        .filter(([, v]) => v)
        .map(([path]) => path);
      const run = await deleteFiles(folder, paths);
      pushToast("success", `Moved ${run.applied_operations.length} file(s) to trash.`);
      await reload();
    } catch (e) {
      pushToast("error", `Delete failed: ${e}`);
    } finally {
      deleting = false;
    }
  }
</script>

<div class="flex items-center gap-2 mb-4">
  <div class="field flex flex-1 items-center gap-2 text-[var(--color-text-muted)]">
    <FolderOpen size={15} class="shrink-0" />
    <span class="truncate {folder ? 'text-[var(--color-text)]' : ''}">
      {folder ?? "No folder selected"}
    </span>
  </div>
  <button type="button" class="btn-primary whitespace-nowrap" onclick={pickFolder}>
    Choose folder…
  </button>
</div>

{#if loading}
  <p class="flex items-center gap-2 text-sm text-[var(--color-text-muted)]">
    <LoaderCircle size={15} class="animate-spin" /> Reading folder…
  </p>
{:else if !folder}
  <div class="flex flex-col items-center gap-2 py-10 text-[var(--color-text-muted)]">
    <FolderSearch size={28} />
    <p class="m-0 text-sm">Pick a sorted folder (e.g. Images, Documents) to see what's in it.</p>
  </div>
{:else if entries.length === 0}
  <p class="py-6 text-sm text-[var(--color-text-muted)]">This folder is empty.</p>
{:else}
  <div class="flex items-center justify-between border-b border-[var(--color-border)] pb-2.5 mb-3">
    <span class="text-sm text-[var(--color-text-muted)]">{entries.length} file(s)</span>
    <button
      type="button"
      class="btn-primary"
      onclick={() => (confirmOpen = true)}
      disabled={deleting || selectedCount === 0}
    >
      {#if deleting}<LoaderCircle size={14} class="animate-spin" />{/if}
      {deleting ? "Deleting…" : `Delete ${selectedCount} file(s)`}
    </button>
  </div>

  {#each groups as [dir, group] (dir)}
    <Collapsible.Root
      class="group mb-2 rounded-md border border-[var(--color-border-subtle)] overflow-hidden"
      open={isGroupOpen(dir)}
      onOpenChange={(v) => (openGroups = { ...openGroups, [dir]: v })}
    >
      <div class="flex items-center gap-2 bg-[var(--color-surface-hover)] px-2.5 py-1.5">
        <Checkbox.Root
          checked={groupChecked(group)}
          indeterminate={groupIndeterminate(group)}
          onCheckedChange={(v) => toggleGroup(group, v === true)}
          class="chk"
        >
          {#snippet children({ checked, indeterminate })}
            {#if indeterminate}<Minus size={11} />{:else if checked}<Check size={11} />{/if}
          {/snippet}
        </Checkbox.Root>
        <Collapsible.Trigger class="group-trigger flex flex-1 items-center gap-1.5 text-left text-sm">
          <ChevronRight size={14} class="chevron shrink-0 text-[var(--color-text-muted)]" />
          <code class="text-[0.8rem]">{dir}</code>
          <span class="text-xs text-[var(--color-text-muted)]">({group.length})</span>
        </Collapsible.Trigger>
      </div>
      <Collapsible.Content>
        <table class="w-full border-collapse text-sm">
          <tbody>
            {#each group as entry (entry.path)}
              <tr class="hover:bg-[var(--color-surface-hover)]">
                <td class="w-8 px-2.5 py-1.5 border-t border-[var(--color-border-subtle)]">
                  <Checkbox.Root
                    checked={selected[entry.path]}
                    onCheckedChange={(v) => (selected = { ...selected, [entry.path]: v === true })}
                    class="chk"
                  >
                    {#snippet children({ checked })}
                      {#if checked}<Check size={11} />{/if}
                    {/snippet}
                  </Checkbox.Root>
                </td>
                <td class="max-w-0 w-[60%] overflow-hidden text-ellipsis whitespace-nowrap px-2.5 py-1.5 border-t border-[var(--color-border-subtle)]" title={entry.path}>
                  {entry.file_name}
                </td>
                <td class="px-2.5 py-1.5 border-t border-[var(--color-border-subtle)] text-[var(--color-text-muted)]">
                  {new Date(entry.modified).toLocaleDateString()}
                </td>
                <td class="px-2.5 py-1.5 border-t border-[var(--color-border-subtle)] text-right whitespace-nowrap">
                  {formatBytes(entry.size_bytes)}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </Collapsible.Content>
    </Collapsible.Root>
  {/each}
{/if}

<DeleteConfirmModal bind:open={confirmOpen} {selectedCount} onConfirm={confirmDelete} onCancel={() => (confirmOpen = false)} />

<style>
  :global(.chk) {
    width: 16px;
    height: 16px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    border: 1px solid var(--color-border);
    background: var(--color-bg);
    color: var(--color-accent-fg);
    padding: 0;
    cursor: pointer;
  }
  :global(.chk[data-state="checked"]),
  :global(.chk[data-state="indeterminate"]) {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
  :global(.chevron) {
    transition: transform 120ms ease;
  }
  :global(.group[data-state="open"] .chevron) {
    transform: rotate(90deg);
  }
</style>
