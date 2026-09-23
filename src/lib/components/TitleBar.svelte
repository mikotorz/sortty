<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Minus from "@lucide/svelte/icons/minus";
  import Square from "@lucide/svelte/icons/square";
  import Copy from "@lucide/svelte/icons/copy";
  import X from "@lucide/svelte/icons/x";

  const appWindow = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    appWindow.isMaximized().then((v) => (maximized = v));
    const unlisten = appWindow.onResized(async () => {
      maximized = await appWindow.isMaximized();
    });
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="brand-name" data-tauri-drag-region>Sortty</span>
  </div>
  <div class="controls">
    <button type="button" class="control" aria-label="Minimize" onclick={() => appWindow.minimize()}>
      <Minus size={14} />
    </button>
    <button type="button" class="control" aria-label={maximized ? "Restore" : "Maximize"} onclick={() => appWindow.toggleMaximize()}>
      {#if maximized}
        <Copy size={13} />
      {:else}
        <Square size={12} />
      {/if}
    </button>
    <button type="button" class="control close" aria-label="Close" onclick={() => appWindow.close()}>
      <X size={15} />
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-height);
    min-height: var(--titlebar-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    user-select: none;
  }
  .brand {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
    padding-left: 0.85rem;
  }
  .brand-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--color-text-muted);
    letter-spacing: 0.02em;
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .control {
    width: 44px;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }
  .control:hover {
    background: var(--color-surface-hover);
    color: var(--color-text);
  }
  .control.close:hover {
    background: var(--color-danger);
    color: var(--color-danger-fg);
  }
</style>
