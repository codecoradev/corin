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
    transition: background-color 0.12s var(--ease-out), border-color 0.12s var(--ease-out), color 0.12s var(--ease-out), opacity 0.12s var(--ease-out);
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

  /* Sizes — min-heights keep every button a comfortable touch target
     (WCAG 2.5.8; primary/per-item actions get the taller 40px row). */
  .btn-sm { padding: 4px 12px; font-size: 0.78rem; min-height: 32px; }
  .btn-md { padding: 7px 14px; font-size: 0.85rem; min-height: 36px; }
  .btn-lg { padding: 10px 20px; font-size: 0.95rem; min-height: 40px; }

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
    border: 1px solid var(--color-red-line);
  }
  .btn-danger:hover:not(:disabled) {
    background: var(--color-red-bg);
    border-color: var(--red);
  }

  .btn-icon-wrap {
    display: flex;
    align-items: center;
  }
</style>
