<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    hoverable?: boolean;
    padding?: 'sm' | 'md' | 'lg' | 'none';
    onclick?: () => void;
    active?: boolean;
    children: Snippet;
  }

  let {
    hoverable = false,
    padding = 'md',
    onclick,
    active = false,
    children,
  }: Props = $props();
</script>

{#if onclick}
  <!-- Clickable cards render a real <button>: Enter/Space and focus come for
       free, and no a11y role/tabindex juggling is needed. -->
  <button
    class="card card-{padding} card-btn"
    class:hoverable
    class:active
    {onclick}
  >
    {@render children()}
  </button>
{:else}
  <div class="card card-{padding}" class:hoverable class:active>
    {@render children()}
  </div>
{/if}

<style>
  .card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color 0.12s ease, transform 0.12s ease;
  }

  .card-sm { padding: 8px 12px; }
  .card-md { padding: 12px 16px; }
  .card-lg { padding: 16px 24px; }
  .card-none { padding: 0; }

  .hoverable {
    cursor: pointer;
  }
  .hoverable:hover {
    border-color: var(--accent);
  }
  .active {
    border-color: var(--accent);
  }

  /* Reset <button> chrome so the clickable card keeps its look. */
  .card-btn {
    display: block;
    width: 100%;
    text-align: left;
    font: inherit;
    color: inherit;
  }
</style>
