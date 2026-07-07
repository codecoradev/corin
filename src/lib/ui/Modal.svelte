<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title?: string;
    onclose: () => void;
    width?: string;
    children: Snippet;
  }

  let { open, title, onclose, width = '500px', children }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="modal-overlay" transition:fade={{ duration: 150 }}>
    <!-- backdrop -->
    <div class="modal-backdrop" onclick={onclose}></div>
    <!-- dialog -->
    <div class="modal-dialog" style="--modal-width: {width}" transition:scale={{ duration: 200, start: 0.96, opacity: 0 }}>
      {#if title}
        <div class="modal-header">
          <h3>{title}</h3>
          <button class="modal-close" onclick={onclose} title="Close (Esc)">✕</button>
        </div>
      {/if}
      <div class="modal-body">
        {@render children()}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
  }
  .modal-dialog {
    position: relative;
    width: var(--modal-width);
    max-width: 92vw;
    max-height: 88vh;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-header h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .modal-close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px 8px;
    border-radius: 4px;
    transition: all 0.12s;
  }
  .modal-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }
</style>
