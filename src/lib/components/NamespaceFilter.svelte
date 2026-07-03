<script lang="ts">
  import { onMount } from 'svelte';
  import { uteke } from '../ts/ipc';

  interface Props {
    /** Currently selected namespaces. Empty/null = all selected. */
    selected: string[];
    onchange: (selected: string[]) => void;
  }

  let { selected, onchange }: Props = $props();

  let allNamespaces = $state<Array<{ name: string; count: number }>>([]);
  let open = $state(false);
  let filterText = $state('');
  let loading = $state(false);

  // The full set of known namespace names — used to compute "all selected".
  let allNames = $derived(allNamespaces.map((n) => n.name));
  let allSelected = $derived(selected.length === 0 || allNames.every((n) => selected.includes(n)));

  let filtered = $derived(
    filterText.trim()
      ? allNamespaces.filter((n) => n.name.toLowerCase().includes(filterText.trim().toLowerCase()))
      : allNamespaces,
  );

  let totalMemories = $derived(allNamespaces.reduce((sum, n) => sum + n.count, 0));

  async function loadNamespaces() {
    loading = true;
    try {
      allNamespaces = await uteke.namespacesWithCounts();
    } catch {
      allNamespaces = [];
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadNamespaces();
  });

  function toggle(ns: string) {
    const set = new Set(selected);
    if (set.has(ns)) set.delete(ns);
    else set.add(ns);
    // Emit the explicit selection. An empty array means "all" semantically,
    // but callers may pass it through as the full set when fanning out.
    onchange([...set]);
  }

  function toggleAll() {
    // If all selected → clear (will read as "none"; we treat empty as all
    // below, so instead selecting-all when none is the useful toggle).
    if (allSelected) {
      // Deselect everything → effectively "none" unless caller maps to all.
      onchange([]);
    } else {
      onchange([...allNames]);
    }
  }

  function handleTriggerClick(e: MouseEvent) {
    e.stopPropagation();
    open = !open;
  }

  function handleDocClick() {
    open = false;
  }

  // Label shown on the trigger button.
  let label = $derived(
    loading
      ? 'Loading…'
      : allSelected || selected.length === 0
        ? `All (${allNames.length})`
        : selected.length === 1
          ? selected[0]
          : `${selected.length} of ${allNames.length}`,
  );

  $effect(() => {
    if (open) {
      // Defer so the opening click doesn't immediately close it.
      setTimeout(() => document.addEventListener('click', handleDocClick), 0);
    } else {
      document.removeEventListener('click', handleDocClick);
    }
  });
</script>

<div class="namespace-filter">
  <button class="trigger" onclick={handleTriggerClick} title="Filter namespaces">
    <span class="ns-icon">◫</span>
    <span class="label">{label}</span>
    <span class="caret">{open ? '▴' : '▾'}</span>
  </button>

  {#if open}
    <div class="dropdown" role="menu" onclick={(e) => e.stopPropagation()}>
      <div class="search">
        <input
          type="text"
          placeholder="Filter namespaces…"
          value={filterText}
          oninput={(e) => (filterText = e.currentTarget.value)}
        />
      </div>

      <button class="select-all" onclick={toggleAll}>
        <span class="check">{allSelected ? '☑' : '☐'}</span>
        <span>{allSelected ? 'Deselect all' : 'Select all'}</span>
        <span class="count">{totalMemories}</span>
      </button>

      <div class="divider"></div>

      <div class="list">
        {#each filtered as ns (ns.name)}
          <label class="ns-row">
            <input
              type="checkbox"
              checked={allSelected || selected.includes(ns.name)}
              onchange={() => toggle(ns.name)}
            />
            <span class="ns-name">{ns.name}</span>
            <span class="count">{ns.count > 0 ? ns.count : '—'}</span>
          </label>
        {:else}
          <div class="empty">{loading ? 'Loading…' : 'No namespaces'}</div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .namespace-filter {
    position: relative;
    display: inline-block;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 5px;
    cursor: pointer;
    font-size: 0.78rem;
    white-space: nowrap;
    transition: border-color 0.1s;
  }

  .trigger:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .ns-icon {
    opacity: 0.7;
  }

  .caret {
    opacity: 0.5;
    font-size: 0.7rem;
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 50;
    width: 260px;
    max-height: 360px;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  .search {
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }

  .search input {
    width: 100%;
    padding: 5px 8px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.8rem;
    outline: none;
  }

  .search input:focus {
    border-color: var(--accent);
  }

  .select-all {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 0.8rem;
    cursor: pointer;
    text-align: left;
  }

  .select-all:hover {
    background: var(--bg-hover);
  }

  .check {
    width: 14px;
    display: inline-block;
  }

  .count {
    margin-left: auto;
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .divider {
    height: 1px;
    background: var(--border);
  }

  .list {
    overflow-y: auto;
    padding: 4px 0;
  }

  .ns-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .ns-row:hover {
    background: var(--bg-hover);
  }

  .ns-row input {
    accent-color: var(--accent);
  }

  .ns-name {
    flex: 1;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    padding: 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.8rem;
  }
</style>
