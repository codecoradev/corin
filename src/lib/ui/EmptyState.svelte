<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    icon?: string;
    title: string;
    subtitle?: string;
    children?: Snippet;
  }

  let { icon = '📭', title, subtitle, children }: Props = $props();
</script>

<div class="empty-state">
  <div class="empty-icon">{icon}</div>
  <p class="empty-title">{title}</p>
  {#if subtitle}
    <p class="empty-subtitle">{subtitle}</p>
  {/if}
  {#if children}
    <div class="empty-action">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    text-align: center;
    gap: 4px;
  }
  .empty-icon {
    font-size: 2.5rem;
    opacity: 0.2;
    margin-bottom: 8px;
  }
  .empty-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .empty-subtitle {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .empty-action {
    margin-top: 12px;
  }
</style>
