<script lang="ts">
  import type { Snippet, Component } from 'svelte';

  interface Props {
    /**
     * Emoji glyph, or an icon component object (e.g. a lucide export —
     * typed loosely as `object` because lucide ships legacy class types).
     */
    icon?: string | object;
    title: string;
    subtitle?: string;
    children?: Snippet;
  }

  let { icon = '📭', title, subtitle, children }: Props = $props();
</script>

<div class="empty-state">
  <div class="empty-icon">
    {#if typeof icon === 'string'}
      {icon}
    {:else}
      {@const Icon = icon as unknown as Component}
      <Icon size={30} strokeWidth={1.5} />
    {/if}
  </div>
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
