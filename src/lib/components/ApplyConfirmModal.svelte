<script lang="ts">
  let {
    open = $bindable(),
    selectedCount,
    onConfirm,
    onCancel,
  }: { open: boolean; selectedCount: number; onConfirm: () => void; onCancel: () => void } = $props();
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="backdrop" role="presentation" onclick={onCancel}>
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <h3>Apply {selectedCount} change{selectedCount === 1 ? "" : "s"}?</h3>
      <p>
        Files will be moved to their new locations now. Nothing is permanently deleted — duplicates
        and stale files go into a <code>.sortty-trash</code> / <code>.sortty-archive</code> folder inside
        the scanned folder (not the Windows Recycle Bin), and this whole run can be undone from
        History afterward.
      </p>
      <div class="actions">
        <button type="button" onclick={onCancel}>Cancel</button>
        <button type="button" class="primary" onclick={onConfirm} disabled={selectedCount === 0}>
          Apply {selectedCount} change{selectedCount === 1 ? "" : "s"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }
  .modal {
    background: var(--card-bg, #fff);
    color: inherit;
    border-radius: 10px;
    padding: 1.5rem;
    max-width: 420px;
    width: 90%;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.25);
  }
  .modal p {
    font-size: 0.9rem;
    line-height: 1.4;
    opacity: 0.85;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1rem;
  }
  button {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: 1px solid var(--border-color, #ccc);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  button.primary {
    background: #396cd8;
    border-color: #396cd8;
    color: white;
  }
  button.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
