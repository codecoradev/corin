<script lang="ts">
  import type { Component } from 'svelte';

  type Variant = 'default' | 'ghost' | 'danger' | 'accent';

  interface Props {
    icon: Component;
    size?: number;
    variant?: Variant;
    onclick?: (e: MouseEvent) => void;
    title?: string;
    disabled?: boolean;
    active?: boolean;
  }

  let {
    icon: Icon,
    size = 16,
    variant = 'default',
    onclick,
    title,
    disabled = false,
    active = false,
  }: Props = $props();
</script>

<button
  class="icon-btn icon-btn-{variant}"
  class:active
  {disabled}
  {onclick}
  {title}
>
  <Icon {size} strokeWidth={2} />
</button>

<style>
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.12s ease;
    flex-shrink: 0;
  }
  .icon-btn:active:not(:disabled) {
    transform: scale(0.92);
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon-btn-default:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .icon-btn-ghost:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .icon-btn-danger {
    color: var(--red);
  }
  .icon-btn-danger:hover:not(:disabled) {
    background: rgba(243, 139, 168, 0.12);
    color: var(--red);
  }

  .icon-btn-accent {
    color: var(--accent);
  }
  .icon-btn-accent:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--accent);
  }

  .active {
    background: var(--bg-hover);
    color: var(--accent);
  }
</style>
