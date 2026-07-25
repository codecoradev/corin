<script lang="ts">
  import type { Snippet } from 'svelte';

  type Color = 'default' | 'accent' | 'green' | 'red' | 'yellow' | 'mauve' | 'teal';

  interface Props {
    color?: Color;
    children: Snippet;
  }

  let { color = 'default', children }: Props = $props();
</script>

<span class="badge badge-{color}">
  {@render children()}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    font-size: 0.68rem;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
    line-height: 1.5;
  }

  .badge-default {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }
  .badge-accent {
    background: var(--color-blue-bg);
    color: var(--accent);
  }
  .badge-green {
    background: var(--color-green-bg);
    color: var(--green);
  }
  .badge-red {
    background: var(--color-red-bg);
    color: var(--red);
  }
  .badge-yellow {
    background: var(--color-yellow-bg);
    color: var(--yellow);
  }
  .badge-mauve {
    background: var(--color-mauve-bg);
    color: var(--mauve);
  }
  .badge-teal {
    background: var(--color-teal-bg);
    color: var(--teal);
  }
</style>
