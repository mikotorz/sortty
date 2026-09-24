<script lang="ts">
  import { fly } from "svelte/transition";
  import { toasts, dismissToast, runToastAction } from "../state/toast";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import Info from "@lucide/svelte/icons/info";
</script>

<div class="toast-stack">
  {#each $toasts as t (t.id)}
    <div
      class="toast {t.kind}"
      role={t.kind === "error" ? "alert" : "status"}
      aria-live={t.kind === "error" ? "assertive" : "polite"}
      transition:fly={{ y: 12, duration: 180 }}
    >
      <button
        type="button"
        class="toast-body"
        onclick={() => dismissToast(t.id)}
      >
        {#if t.kind === "success"}
          <CircleCheck size={16} />
        {:else if t.kind === "error"}
          <CircleX size={16} />
        {:else}
          <Info size={16} />
        {/if}
        <span>{t.message}</span>
      </button>
      {#if t.action}
        <button
          type="button"
          class="toast-action"
          onclick={() => runToastAction(t.id)}
        >
          {t.action.label}
        </button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 50;
    max-width: 360px;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.65rem 0.9rem;
    border-radius: 8px;
    font-size: 0.85rem;
    text-align: left;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
    background: var(--color-surface);
    color: var(--color-text);
    border: 1px solid var(--color-border);
  }
  .toast-body {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex: 1;
    text-align: left;
    cursor: pointer;
    color: inherit;
  }
  .toast-action {
    flex-shrink: 0;
    padding: 0.2rem 0.6rem;
    border-radius: 6px;
    border: 1px solid currentColor;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    color: inherit;
  }
  .toast.success {
    background: var(--color-success-bg);
    color: var(--color-success-fg);
    border-color: transparent;
  }
  .toast.error {
    background: var(--color-danger);
    color: var(--color-danger-fg);
    border-color: transparent;
  }
  .toast.info {
    background: var(--color-surface);
    color: var(--color-text);
  }
</style>
