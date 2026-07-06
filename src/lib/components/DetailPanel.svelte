<script lang="ts">
  import { onMount, onDestroy, type Snippet } from 'svelte';

  interface Props {
    memoryId: string;
    onclose: () => void;
    onneighborclick?: (id: string) => void;
    onedit?: (m: any) => void;
    children: Snippet;
  }

  let { memoryId, onclose, onneighborclick, onedit, children }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });
  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<!-- Backdrop (click to close) -->
<div
  class="detail-backdrop"
  onclick={onclose}
  role="presentation"
></div>

<!-- Slide-in panel -->
<aside class="detail-panel" role="dialog" aria-modal="true">
  {@render children?.()}
</aside>

<style>
  .detail-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.3);
    z-index: 89;
    animation: fadeIn 0.15s ease;
  }

  .detail-panel {
    position: fixed;
    top: 0;
    right: 0;
    width: 500px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    box-shadow: -8px 0 32px rgba(0, 0, 0, 0.35);
    z-index: 90;
    /* No overflow on the panel itself — prevents clipping the
       delete confirmation dialog. MemoryDetail scrolls internally. */
    animation: slideIn 0.2s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  /* Animate via right offset, NOT transform — transform creates a
     stacking context that traps child dialogs below the panel. */
  @keyframes slideIn {
    from { right: -500px; }
    to   { right: 0; }
  }
</style>
