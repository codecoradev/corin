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

  let panelEl: HTMLElement | null = null;
  let previouslyFocused: HTMLElement | null = null;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
      return;
    }
    // Focus trap: cycle Tab/Shift+Tab within the dialog.
    if (e.key !== 'Tab' || !panelEl) return;
    const focusable = panelEl.querySelectorAll<HTMLElement>(
      'button, [href], input, textarea, select, [tabindex]:not([tabindex="-1"])',
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey) {
      if (document.activeElement === first || !panelEl!.contains(document.activeElement)) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  onMount(() => {
    previouslyFocused = document.activeElement as HTMLElement;
    window.addEventListener('keydown', handleKeydown);
    // Move focus into the panel on open so screen readers announce it.
    if (panelEl) {
      const first = panelEl.querySelector<HTMLElement>(
        'button, [href], input, [tabindex]:not([tabindex="-1"])',
      );
      if (first) {
        first.focus();
      } else {
        panelEl.focus();
      }
    }
  });
  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    // Restore focus to the element that opened the panel.
    previouslyFocused?.focus();
  });
</script>

<!-- Backdrop (click to close) -->
<div
  class="detail-backdrop"
  onclick={onclose}
  role="presentation"
></div>

<!-- Slide-in panel -->
<div class="detail-panel" bind:this={panelEl} role="dialog" aria-modal="true" tabindex="-1">
  {@render children?.()}
</div>

<style>
  .detail-backdrop {
    position: fixed;
    inset: 0;
    background: var(--scrim);
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
