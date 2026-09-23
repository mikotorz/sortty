<script lang="ts">
  import "../app.css";
  import "$lib/state/theme";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ToastStack from "$lib/components/ToastStack.svelte";

  let { children } = $props();
</script>

<div class="app-shell">
  <TitleBar />
  <div class="app-body">
    <Sidebar />
    <main class="app-content">
      {@render children()}
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
