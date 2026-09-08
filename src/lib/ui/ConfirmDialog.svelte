<script lang="ts">
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title?: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
    confirmDisabled?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
    /** Extra content below the message (e.g. a warning paragraph). */
    children?: Snippet;
  }

  let {
    open,
    title = 'Confirm',
    message,
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    danger = false,
    confirmDisabled = false,
    onconfirm,
    oncancel,
    children,
  }: Props = $props();
</script>

<Modal {open} {title} onclose={oncancel} width="400px">
  <p class="confirm-msg">{message}</p>
  {#if children}
    {@render children()}
  {/if}
  <div class="confirm-actions">
    <Button variant="ghost" size="md" onclick={oncancel}>{cancelLabel}</Button>
    <Button variant={danger ? 'danger' : 'primary'} size="md" onclick={onconfirm} disabled={confirmDisabled}>{confirmLabel}</Button>
  </div>
</Modal>

<style>
  .confirm-msg {
    margin: 0 0 20px;
    font-size: 0.9rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
