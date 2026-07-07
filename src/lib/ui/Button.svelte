<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Component } from 'svelte';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    variant?: Variant;
    size?: Size;
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    title?: string;
    type?: 'button' | 'submit' | 'reset';
    children: Snippet;
    icon?: Component;
  }

  let {
    variant = 'secondary',
    size = 'md',
    disabled = false,
    onclick,
    title,
    type = 'button',
    children,
    icon: Icon,
  }: Props = $props();
</script>

<button
  class="btn btn-{variant} btn-{size}"
  {disabled}
  {onclick}
  {title}
  {type}
>
  {#if Icon}
    <span class="btn-icon-wrap">
      <Icon size={size === 'sm' ? 14 : 16} strokeWidth={2} />
    </span>
  {/if}
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-family: inherit;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.12s ease;
    user-select: none;
    line-height: 1;
  }
  .btn:active:not(:disabled) {
    transform: scale(0.97);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Sizes */
  .btn-sm { padding: 4px 10px; font-size: 0.78rem; }
  .btn-md { padding: 7px 14px; font-size: 0.85rem; }
  .btn-lg { padding: 10px 20px; font-size: 0.95rem; }

  /* Variants */
  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }
  .btn-primary:hover:not(:disabled) { opacity: 0.88; }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }
  .btn-secondary:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-danger {
    background: transparent;
    color: var(--red);
    border: 1px solid rgba(243, 139, 168, 0.3);
  }
  .btn-danger:hover:not(:disabled) {
    background: rgba(243, 139, 168, 0.1);
    border-color: var(--red);
  }

  .btn-icon-wrap {
    display: flex;
    align-items: center;
  }
</style>
