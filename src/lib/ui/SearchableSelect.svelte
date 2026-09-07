<script lang="ts">
  /**
   * Searchable single-select — a native <select> replacement for long lists
   * (e.g. export namespaces): type-to-filter, ↑↓/Enter keyboard navigation,
   * click-outside or Esc to close. The '' value renders `emptyLabel`.
   */
  interface Props {
    options?: string[];
    value?: string;
    /** Label shown for the '' value. */
    emptyLabel?: string;
    placeholder?: string;
    disabled?: boolean;
  }

  let { options = [], value = $bindable(''), emptyLabel = 'All', placeholder = 'Search…', disabled = false }: Props = $props();

  let root = $state<HTMLElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let open = $state(false);
  let query = $state('');
  let active = $state(0);
  const listId = `ss-list-${Math.random().toString(36).slice(2, 8)}`;

  interface Opt {
    value: string;
    label: string;
  }

  let candidates = $derived.by<Opt[]>(() => {
    const all: Opt[] = [{ value: '', label: emptyLabel }, ...options.map((o) => ({ value: o, label: o }))];
    const q = query.trim().toLowerCase();
    return q ? all.filter((o) => o.label.toLowerCase().includes(q)) : all;
  });

  let selectedLabel = $derived(value === '' ? emptyLabel : value);

  // Closed state mirrors the selection into the input (combobox display).
  $effect(() => {
    if (!open) query = selectedLabel;
  });

  function openList() {
    if (disabled) return;
    open = true;
    query = '';
    const idx = candidates.findIndex((o) => o.value === value);
    active = Math.max(0, idx);
  }

  function choose(o: Opt) {
    value = o.value;
    open = false;
    inputEl?.blur();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'Enter' || e.key === 'ArrowDown') {
        e.preventDefault();
        openList();
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(active + 1, candidates.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const o = candidates[active];
      if (o) choose(o);
    } else if (e.key === 'Escape') {
      // Keep the keystroke from closing the settings modal behind us.
      e.preventDefault();
      e.stopPropagation();
      open = false;
    }
  }

  function onWindowPointerDown(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<div class="ss" bind:this={root}>
  <input
    bind:this={inputEl}
    type="text"
    bind:value={query}
    placeholder={open ? placeholder : selectedLabel}
    disabled={disabled}
    role="combobox"
    aria-expanded={open}
    aria-controls={open ? listId : undefined}
    onfocus={openList}
    oninput={() => {
      open = true;
      active = 0;
    }}
    onkeydown={handleKeydown}
  />

  {#if open}
    <ul class="ss-list" role="listbox" id={listId}>
      {#each candidates as o, i (o.value || '__empty__')}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={o.value === value}
            class:ss-on={i === active}
            class:ss-selected={o.value === value}
            onmouseenter={() => (active = i)}
            onclick={() => choose(o)}
          >
            {o.label}
          </button>
        </li>
      {:else}
        <li class="ss-empty">No matches.</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .ss {
    position: relative;
  }
  .ss::after {
    content: '▾';
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    font-size: 0.7rem;
    pointer-events: none;
  }
  .ss input {
    width: 100%;
    padding: 10px 32px 10px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 0.88rem;
    outline: none;
    cursor: pointer;
  }
  .ss input:focus {
    border-color: var(--accent);
    cursor: text;
  }
  .ss input::placeholder {
    color: var(--text-secondary);
  }
  .ss input:disabled {
    opacity: 0.5;
  }
  .ss-list {
    position: absolute;
    z-index: 20;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    max-height: 240px;
    overflow-y: auto;
  }
  .ss-list button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.85rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .ss-list button.ss-on {
    background: var(--bg-hover);
  }
  .ss-list button.ss-selected {
    color: var(--accent);
    font-weight: 600;
  }
  .ss-empty {
    padding: 8px 10px;
    color: var(--text-muted);
    font-size: 0.8rem;
  }
</style>
