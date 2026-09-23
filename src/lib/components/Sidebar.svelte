<script lang="ts">
  import { page } from "$app/stores";
  import WandTwo from "@lucide/svelte/icons/wand-2";
  import History from "@lucide/svelte/icons/history";
  import Settings from "@lucide/svelte/icons/settings";

  const items = [
    { href: "/", label: "Sort & Clean", icon: WandTwo },
    { href: "/history", label: "History", icon: History },
    { href: "/settings", label: "Settings", icon: Settings },
  ];
</script>

<nav class="sidebar">
  {#each items as item (item.href)}
    <a
      class="nav-item"
      class:active={$page.url.pathname === item.href}
      href={item.href}
    >
      <item.icon size={17} />
      <span>{item.label}</span>
    </a>
  {/each}
</nav>

<style>
  .sidebar {
    width: var(--sidebar-width);
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.75rem 0.6rem;
    border-right: 1px solid var(--color-border);
    background: var(--color-surface);
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.65rem;
    border-radius: 8px;
    color: var(--color-text-muted);
    text-decoration: none;
    font-size: 0.88rem;
  }
  .nav-item:hover {
    background: var(--color-surface-hover);
    color: var(--color-text);
  }
  .nav-item.active {
    background: var(--color-accent);
    color: var(--color-accent-fg);
  }
</style>
