<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { selectedRoot } from "../state/stores";
  import FolderOpen from "@lucide/svelte/icons/folder-open";

  async function pickFolder() {
    const path = await open({ directory: true, multiple: false, title: "Choose a folder to sort" });
    if (typeof path === "string") {
      selectedRoot.set(path);
    }
  }
</script>

<div class="flex items-center gap-2">
  <div class="field flex flex-1 items-center gap-2 text-[var(--color-text-muted)]">
    <FolderOpen size={15} class="shrink-0" />
    <span class="truncate {$selectedRoot ? 'text-[var(--color-text)]' : ''}">
      {$selectedRoot ?? "No folder selected"}
    </span>
  </div>
  <button type="button" class="btn-primary whitespace-nowrap" onclick={pickFolder}>
    Choose folder…
  </button>
</div>
