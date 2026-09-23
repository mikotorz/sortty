<script lang="ts">
  import "../app.css";
  import "$lib/state/theme";
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ToastStack from "$lib/components/ToastStack.svelte";
  import { getSettings } from "$lib/api/commands";
  import { appSettings, selectedRoot } from "$lib/state/stores";
  import { scanSession } from "$lib/state/scanSession.svelte";
  import {
    isPlanRequestMode,
    defaultRequestForMode,
  } from "$lib/state/planRequestDefaults";
  import { scheduleGeneralSave } from "$lib/state/generalSettingsSync";
  import { createScrollRoot } from "$lib/state/scrollRoot.svelte";

  let { children } = $props();

  // Lists virtualize against the page's own scroller, so the whole page
  // scrolls to the end rather than a box inside it (see ADR 0015). The
  // observer watches the page content, not the fixed-size scroller, so it
  // fires on anything that can shift a list's offset.
  const scrollRoot = createScrollRoot();
  let pageContentEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const el = pageContentEl;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      scrollRoot.layoutVersion++;
    });
    ro.observe(el);
    return () => ro.disconnect();
  });

  // Remember the last folder and mode across restarts. Hydration is guarded
  // against clobbering a folder/mode the user already picked before settings
  // finished loading, and persistence is suppressed until hydration is done
  // so loading the saved values doesn't immediately re-save them.
  let hydrating = $state(true);

  onMount(async () => {
    try {
      const settings = await getSettings();
      appSettings.set(settings);
      if (get(selectedRoot) === null && settings.general.default_root) {
        selectedRoot.set(settings.general.default_root);
      }
      if (
        scanSession.request.mode === "sort_by_type" &&
        isPlanRequestMode(settings.general.last_used_mode) &&
        settings.general.last_used_mode !== "sort_by_type"
      ) {
        scanSession.request = defaultRequestForMode(
          settings.general.last_used_mode,
          settings,
        );
      }
    } finally {
      hydrating = false;
    }
  });

  selectedRoot.subscribe((root) => {
    if (hydrating) return;
    scheduleGeneralSave({ default_root: root });
  });

  $effect(() => {
    const mode = scanSession.request.mode;
    if (hydrating) return;
    scheduleGeneralSave({ last_used_mode: mode });
  });
</script>

<div class="app-shell">
  <TitleBar />
  <div class="app-body">
    <Sidebar />
    <main class="app-content" bind:this={scrollRoot.el}>
      <div bind:this={pageContentEl}>
        {@render children()}
      </div>
    </main>
  </div>
</div>

<ToastStack />

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .app-body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .app-content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 1.5rem;
  }
</style>
