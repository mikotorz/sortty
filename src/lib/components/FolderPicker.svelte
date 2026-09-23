<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { selectedRoot } from "../state/stores";

  async function pickFolder() {
    const path = await open({ directory: true, multiple: false, title: "Choose a folder to sort" });
    if (typeof path === "string") {
      selectedRoot.set(path);
    }
  }
</script>

<div class="folder-picker">
  <input
    class="folder-input"
    type="text"
    readonly
    placeholder="No folder selected"
    value={$selectedRoot ?? ""}
  />
  <button type="button" onclick={pickFolder}>Choose folder…</button>
</div>

<style>
  .folder-picker {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .folder-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    border: 1px solid var(--border-color, #ccc);
    background: var(--input-bg, #fff);
    color: inherit;
  }
  button {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #396cd8;
    color: white;
    cursor: pointer;
    white-space: nowrap;
  }
  button:hover {
    background: #2d55ab;
  }
</style>
