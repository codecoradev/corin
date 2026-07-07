<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    value?: string;
    placeholder?: string;
    type?: string;
    bind?: 'value';
    oninput?: (e: Event) => void;
    onkeydown?: (e: KeyboardEvent) => void;
    title?: string;
    children?: Snippet;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    type = 'text',
    oninput,
    onkeydown,
    title,
    children,
  }: Props = $props();
</script>

<div class="input-wrap">
  {#if children}
    <span class="input-prefix">
      {@render children()}
    </span>
  {/if}
  <input
    class="ui-input"
    {type}
    {placeholder}
    {title}
    bind:value
    {oninput}
    {onkeydown}
  />
</div>

<style>
  .input-wrap {
    display: flex;
    align-items: center;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color 0.12s ease;
  }
  .input-wrap:focus-within {
    border-color: var(--accent);
  }
  .input-prefix {
    display: flex;
    align-items: center;
    padding-left: 8px;
    color: var(--text-muted);
  }
  .ui-input {
    flex: 1;
    min-width: 0;
    padding: 7px 10px;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 0.85rem;
    font-family: inherit;
  }
  .ui-input::placeholder {
    color: var(--text-muted);
  }
</style>
