<script lang="ts">
  import { onMount } from 'svelte';
  import { uteke } from '../ts/ipc';

  interface Props {
    /**
     * Selected namespaces.
     * - `null`  → all selected (default; show every namespace)
     * - `[]`    → none selected (show nothing)
     * - `[...]` → explicit selection
     */
    selected: string[] | null;
    onchange: (selected: string[] | null) => void;
  }

  let { selected, onchange }: Props = $props();

  let allNamespaces = $state<Array<{ name: string; count: number }>>([]);
  let open = $state(false);
  let filterText = $state('');
  let loading = $state(false);

  let allNames = $derived(allNamespaces.map((n) => n.name));

  // Resolve the effective selection: null (= all) becomes the full list so
  // checkbox state and toggling work on a concrete set.
  let effective = $derived<string[]>(selected === null ? allNames : selected);

  let allSelected = $derived(allNames.length > 0 && effective.length === allNames.length);
  let noneSelected = $derived(selected !== null && selected.length === 0);

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

  // Toggle a single namespace. Starts from the effective (resolved) set so
  // unchecking one item from the "all" state removes just that item, instead
  // of flipping to "only this item checked".
  function toggle(ns: string) {
    const set = new Set(effective);
    if (set.has(ns)) set.delete(ns);
    else set.add(ns);
    const arr = [...set];
    // If we're back to the full set, normalize to null (= all) so the parent
    // treats it as "no filter".
    onchange(arr.length === allNames.length && allNames.length > 0 ? null : arr);
  }

  function toggleAll() {
    if (allSelected) {
      // all → none
      onchange([]);
    } else {
      // none or partial → all
      onchange(null);
    }
  }

  function handleTriggerClick(e: MouseEvent) {
    e.stopPropagation();
    open = !open;
  }

  function handleDocClick() {
    open = false;
  }

  let label = $derived.by(() => {
    if (loading) return 'Loading…';
    if (allNames.length === 0) return 'Namespaces';
    if (selected === null || allSelected) return `All (${allNames.length})`;
    if (noneSelected) return 'None';
    if (selected.length === 1) return selected[0];
    return `${selected.length} of ${allNames.length}`;
  });

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
    <!-- svelte-ignore a11y_click_events_have_key_events -->
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
        <span class="check">{allSelected ? '☑' : noneSelected ? '☐' : '⊟'}</span>
        <span>{allSelected ? 'Deselect all' : noneSelected ? 'Select all' : 'Select all'}</span>
        <span class="count">{totalMemories}</span>
      </button>

      <div class="divider"></div>

      <div class="list">
        {#each filtered as ns (ns.name)}
          <label class="ns-row">
            <input
              type="checkbox"
              checked={effective.includes(ns.name)}
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
