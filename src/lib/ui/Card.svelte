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

<div
  class="card card-{padding}"
  class:hoverable
  class:active
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  {onclick}
  onkeydown={(e) => onclick && (e.key === 'Enter' && onclick())}
>
  {@render children()}
</div>

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
</style>
