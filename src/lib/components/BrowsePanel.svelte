<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { Collapsible, Checkbox } from "bits-ui";
  import {
    browseFolder,
    deleteFiles,
    emptyStagingFolder,
    previewStagingFolder,
  } from "../api/commands";
  import type { EmptyResult, FileEntry, StagingKind } from "../api/types";
  import { formatBytes } from "../format";
  import { pushToast } from "../state/toast";
  import { getScrollRoot } from "../state/scrollRoot.svelte";
  import {
    groupByDir,
    isGroupChecked,
    isGroupIndeterminate,
    withGroupSelection,
  } from "../grouping";
  import ConfirmModal from "./ConfirmModal.svelte";
  import VirtualList from "./VirtualList.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Check from "@lucide/svelte/icons/check";
  import Minus from "@lucide/svelte/icons/minus";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Archive from "@lucide/svelte/icons/archive";

  const idOf = (e: FileEntry) => e.path;

  let folder = $state<string | null>(null);
  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let deleting = $state(false);
  let confirmOpen = $state(false);
  let selected = $state<Record<string, boolean>>({});
  let openGroups = $state<Record<string, boolean>>({});

  const scrollRoot = getScrollRoot();

  let emptyConfirmOpen = $state(false);
  let emptyKind = $state<StagingKind | null>(null);
  let emptyPreview = $state<EmptyResult | null>(null);
  let emptying = $state(false);

  let groups = $derived(groupByDir(entries, idOf));

  let selectedCount = $derived(Object.values(selected).filter(Boolean).length);

  function isGroupOpen(dir: string): boolean {
    return openGroups[dir] ?? true;
  }

  function toggleGroup(group: FileEntry[], value: boolean) {
    selected = withGroupSelection(group, idOf, selected, value);
  }

  async function pickFolder() {
    const path = await open({
      directory: true,
      multiple: false,
      title: "Choose a sorted folder to browse",
    });
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
      pushToast(
        "success",
        `Moved ${run.applied_operations.length} file(s) to trash.`,
      );
      await reload();
    } catch (e) {
      pushToast("error", `Delete failed: ${e}`);
    } finally {
      deleting = false;
    }
  }

  async function startEmpty(kind: StagingKind) {
    if (!folder) return;
    const noun = kind === "trash" ? "trash" : "archive";
    try {
      const preview = await previewStagingFolder(folder, kind);
      if (preview.deleted_files === 0) {
        pushToast("info", `There's nothing in the ${noun} to empty.`);
        return;
      }
      emptyKind = kind;
      emptyPreview = preview;
      emptyConfirmOpen = true;
    } catch (e) {
      pushToast("error", `Couldn't check the ${noun}: ${e}`);
    }
  }

  async function confirmEmpty() {
    if (!folder || !emptyKind) return;
    const kind = emptyKind;
    const noun = kind === "trash" ? "trash" : "archive";
    emptyConfirmOpen = false;
    emptying = true;
    try {
      const result = await emptyStagingFolder(folder, kind);
      pushToast(
        "success",
        `Permanently deleted ${result.deleted_files} file(s) from the ${noun}, freed ${formatBytes(result.freed_bytes)}.`,
      );
      await reload();
    } catch (e) {
      pushToast("error", `Couldn't empty the ${noun}: ${e}`);
    } finally {
      emptying = false;
      emptyKind = null;
      emptyPreview = null;
    }
  }
</script>

<div class="flex items-center gap-2 mb-4">
  <div
    class="field flex flex-1 items-center gap-2 text-[var(--color-text-muted)]"
  >
    <FolderOpen size={15} class="shrink-0" />
    <span class="truncate {folder ? 'text-[var(--color-text)]' : ''}">
      {folder ?? "No folder selected"}
    </span>
  </div>
  <button
    type="button"
    class="btn-primary whitespace-nowrap"
    onclick={pickFolder}
  >
    Choose folder…
  </button>
</div>

{#if folder}
  <div class="flex items-center gap-2 mb-4">
    <button
      type="button"
      class="btn-ghost flex items-center gap-1.5 px-3 py-1.5 text-sm"
      onclick={() => startEmpty("trash")}
      disabled={emptying}
    >
      <Trash2 size={14} /> Empty Trash…
    </button>
    <button
      type="button"
      class="btn-ghost flex items-center gap-1.5 px-3 py-1.5 text-sm"
      onclick={() => startEmpty("archive")}
      disabled={emptying}
    >
      <Archive size={14} /> Empty Archive…
    </button>
  </div>
{/if}

{#if loading}
  <p class="flex items-center gap-2 text-sm text-[var(--color-text-muted)]">
    <LoaderCircle size={15} class="animate-spin" /> Reading folder…
  </p>
{:else if !folder}
  <div
    class="flex flex-col items-center gap-2 py-10 text-[var(--color-text-muted)]"
  >
    <FolderSearch size={28} />
    <p class="m-0 text-sm">
      Pick a sorted folder (e.g. Images, Documents) to see what's in it.
    </p>
  </div>
{:else if entries.length === 0}
  <p class="py-6 text-sm text-[var(--color-text-muted)]">
    This folder is empty.
  </p>
{:else}
  <div
    class="flex items-center justify-between border-b border-[var(--color-border)] pb-2.5 mb-3"
  >
    <span class="text-sm text-[var(--color-text-muted)]"
      >{entries.length} file(s)</span
    >
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
      <div
        class="flex items-center gap-2 bg-[var(--color-surface-hover)] px-2.5 py-1.5"
      >
        <Checkbox.Root
          checked={isGroupChecked(group, idOf, selected)}
          indeterminate={isGroupIndeterminate(group, idOf, selected)}
          onCheckedChange={(v) => toggleGroup(group, v === true)}
          class="chk"
        >
          {#snippet children({ checked, indeterminate })}
            {#if indeterminate}<Minus size={11} />{:else if checked}<Check
                size={11}
              />{/if}
          {/snippet}
        </Checkbox.Root>
        <Collapsible.Trigger
          class="group-trigger flex flex-1 items-center gap-1.5 text-left text-sm"
        >
          <ChevronRight
            size={14}
            class="chevron shrink-0 text-[var(--color-text-muted)]"
          />
          <code class="text-[0.8rem]">{dir}</code>
          <span class="text-xs text-[var(--color-text-muted)]"
            >({group.length})</span
          >
        </Collapsible.Trigger>
      </div>
      <Collapsible.Content>
        <div class="text-sm" role="table">
          <VirtualList
            items={group}
            estimateSize={37}
            scrollElement={scrollRoot.el}
            layoutVersion={scrollRoot.layoutVersion}
          >
            {#snippet row(entry: FileEntry)}
              <div
                role="row"
                class="grid h-full items-center gap-2.5 border-t border-[var(--color-border-subtle)] px-2.5 hover:bg-[var(--color-surface-hover)]"
                style="grid-template-columns: 2rem minmax(0, 60%) minmax(0, 1fr) auto;"
              >
                <Checkbox.Root
                  checked={selected[entry.path]}
                  onCheckedChange={(v) =>
                    (selected = { ...selected, [entry.path]: v === true })}
                  class="chk"
                >
                  {#snippet children({ checked })}
                    {#if checked}<Check size={11} />{/if}
                  {/snippet}
                </Checkbox.Root>
                <span
                  class="overflow-hidden text-ellipsis whitespace-nowrap"
                  title={entry.path}
                >
                  {entry.file_name}
                </span>
                <span
                  class="overflow-hidden text-ellipsis whitespace-nowrap text-[var(--color-text-muted)]"
                >
                  {new Date(entry.modified).toLocaleDateString()}
                </span>
                <span class="whitespace-nowrap text-right"
                  >{formatBytes(entry.size_bytes)}</span
                >
              </div>
            {/snippet}
          </VirtualList>
        </div>
      </Collapsible.Content>
    </Collapsible.Root>
  {/each}
{/if}

<ConfirmModal
  bind:open={confirmOpen}
  title="Delete {selectedCount} file{selectedCount === 1 ? '' : 's'}?"
  confirmLabel="Delete {selectedCount} file{selectedCount === 1 ? '' : 's'}"
  disabled={selectedCount === 0}
  onConfirm={confirmDelete}
  onCancel={() => (confirmOpen = false)}
>
  {#snippet description()}
    Nothing is permanently deleted — these files move into a <code
      >.sortty-trash</code
    > folder next to them (not the Windows Recycle Bin), and this can be undone from
    History afterward.
  {/snippet}
</ConfirmModal>

<ConfirmModal
  bind:open={emptyConfirmOpen}
  title={emptyKind === "trash"
    ? "Permanently delete trash contents?"
    : "Permanently delete archived files?"}
  confirmLabel="Delete permanently"
  disabled={emptying}
  onConfirm={confirmEmpty}
  onCancel={() => {
    emptyConfirmOpen = false;
    emptyKind = null;
    emptyPreview = null;
  }}
>
  {#snippet description()}
    {#if emptyPreview}
      This will permanently delete <strong
        >{emptyPreview.deleted_files} file{emptyPreview.deleted_files === 1
          ? ""
          : "s"}</strong
      >
      ({formatBytes(emptyPreview.freed_bytes)}). Unlike everything else in
      sortty, this cannot be undone.
    {/if}
  {/snippet}
</ConfirmModal>
