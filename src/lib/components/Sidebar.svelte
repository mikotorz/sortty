<script lang="ts">
  import { page } from "$app/stores";
  import { resolve } from "$app/paths";
  import WandTwo from "@lucide/svelte/icons/wand-2";
  import History from "@lucide/svelte/icons/history";
  import Settings from "@lucide/svelte/icons/settings";
  import FolderOpen from "@lucide/svelte/icons/folder-open";

  const items = [
    { href: "/", label: "Sort & Clean", icon: WandTwo },
    { href: "/browse", label: "Browse", icon: FolderOpen },
    { href: "/history", label: "History", icon: History },
    { href: "/settings", label: "Settings", icon: Settings },
  ] as const;
</script>

<nav class="sidebar">
  {#each items as item (item.href)}
    {@const isActive = $page.url.pathname === item.href}
    <a
      class="nav-item"
      class:active={isActive}
      aria-current={isActive ? "page" : undefined}
      href={resolve(item.href)}
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
