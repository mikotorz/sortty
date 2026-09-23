<script lang="ts">
  import { Dialog } from "bits-ui";
  import { fade, fly } from "svelte/transition";

  let {
    open = $bindable(),
    selectedCount,
    onConfirm,
    onCancel,
  }: { open: boolean; selectedCount: number; onConfirm: () => void; onCancel: () => void } = $props();
</script>

<Dialog.Root bind:open onOpenChange={(v) => !v && onCancel()}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-40 bg-black/40" forceMount>
      {#snippet child({ props, open })}
        {#if open}
          <div {...props} transition:fade={{ duration: 120 }}></div>
        {/if}
      {/snippet}
    </Dialog.Overlay>
    <Dialog.Content
      class="fixed left-1/2 top-1/2 z-50 w-[90%] max-w-[420px] -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-card)] bg-[var(--color-surface)] p-6 shadow-2xl outline-none"
      forceMount
    >
      {#snippet child({ props, open })}
        {#if open}
          <div {...props} transition:fly={{ y: 8, duration: 140 }}>
            <Dialog.Title class="text-base font-semibold m-0">
              Apply {selectedCount} change{selectedCount === 1 ? "" : "s"}?
            </Dialog.Title>
            <Dialog.Description class="mt-2 text-sm leading-relaxed text-[var(--color-text-muted)]">
              Files will be moved to their new locations now. Nothing is permanently deleted — duplicates
              and stale files go into a <code>.sortty-trash</code> / <code>.sortty-archive</code> folder inside
              the scanned folder (not the Windows Recycle Bin), and this whole run can be undone from
              History afterward.
            </Dialog.Description>
            <div class="mt-4 flex justify-end gap-2">
              <button type="button" class="btn-ghost" onclick={onCancel}>
                Cancel
              </button>
              <button
                type="button"
                class="btn-primary"
                onclick={onConfirm}
                disabled={selectedCount === 0}
              >
                Apply {selectedCount} change{selectedCount === 1 ? "" : "s"}
              </button>
            </div>
          </div>
        {/if}
      {/snippet}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
