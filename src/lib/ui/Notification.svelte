<script lang="ts">
  import { fly } from 'svelte/transition';
  import { X } from 'lucide-svelte';

  type ToastType = 'success' | 'error' | 'info';

  interface Toast {
    id: number;
    type: ToastType;
    message: string;
  }

  interface Props {
    toasts: Toast[];
    ondismiss: (id: number) => void;
  }

  let { toasts, ondismiss }: Props = $props();

  const icons: Record<ToastType, string> = {
    success: '✓',
    error: '✕',
    info: 'ℹ',
  };
</script>

<div class="toast-container">
  {#each toasts as toast (toast.id)}
    <div
      class="toast toast-{toast.type}"
      transition:fly={{ y: -16, duration: 200 }}
    >
      <span class="toast-icon">{icons[toast.type]}</span>
      <span class="toast-msg">{toast.message}</span>
      <button class="toast-dismiss" onclick={() => ondismiss(toast.id)}>
        <X size={13} strokeWidth={2.5} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    top: 12px;
    right: 12px;
    z-index: 2000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: var(--radius);
    font-size: 0.85rem;
    font-weight: 500;
    min-width: 240px;
    max-width: 380px;
    pointer-events: auto;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }
  .toast-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    font-size: 0.65rem;
    font-weight: 700;
    flex-shrink: 0;
  }
  .toast-msg {
    flex: 1;
    min-width: 0;
  }
  .toast-dismiss {
    background: none;
    border: none;
    color: inherit;
    opacity: 0.5;
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    transition: opacity 0.12s;
  }
  .toast-dismiss:hover { opacity: 1; }

  .toast-success {
    background: rgba(166, 227, 161, 0.12);
    border: 1px solid rgba(166, 227, 161, 0.3);
    color: var(--green);
  }
  .toast-success .toast-icon { background: var(--green); color: var(--bg-primary); }

  .toast-error {
    background: rgba(243, 139, 168, 0.12);
    border: 1px solid rgba(243, 139, 168, 0.3);
    color: var(--red);
  }
  .toast-error .toast-icon { background: var(--red); color: var(--bg-primary); }

  .toast-info {
    background: rgba(137, 180, 250, 0.12);
    border: 1px solid rgba(137, 180, 250, 0.3);
    color: var(--accent);
  }
  .toast-info .toast-icon { background: var(--accent); color: var(--bg-primary); }
</style>
