<script lang="ts">
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';

  interface Props {
    open: boolean;
    title?: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let {
    open,
    title = 'Confirm',
    message,
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    danger = false,
    onconfirm,
    oncancel,
  }: Props = $props();
</script>

<Modal {open} {title} onclose={oncancel} width="400px">
  <p class="confirm-msg">{message}</p>
  <div class="confirm-actions">
    <Button variant="ghost" size="md" onclick={oncancel}>{cancelLabel}</Button>
    <Button variant={danger ? 'danger' : 'primary'} size="md" onclick={onconfirm}>{confirmLabel}</Button>
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
