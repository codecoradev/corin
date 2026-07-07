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
    background: rgba(137, 180, 250, 0.15);
    color: var(--accent);
  }
  .badge-green {
    background: rgba(166, 227, 161, 0.15);
    color: var(--green);
  }
  .badge-red {
    background: rgba(243, 139, 168, 0.15);
    color: var(--red);
  }
  .badge-yellow {
    background: rgba(249, 226, 175, 0.15);
    color: var(--yellow);
  }
  .badge-mauve {
    background: rgba(203, 166, 247, 0.15);
    color: var(--mauve);
  }
  .badge-teal {
    background: rgba(148, 226, 213, 0.15);
    color: var(--teal);
  }
</style>
