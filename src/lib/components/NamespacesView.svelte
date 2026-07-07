<script lang="ts">
  import { uteke } from '../ts/ipc';
  import { createPager } from '../stores/pagination.svelte';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    onmemoryclick: (id: string) => void;
  }

  let { onmemoryclick }: Props = $props();

  let namespaces = $state<{ name: string; count: number }[]>([]);
  let loading = $state(true);
  let selectedNs = $state<string | null>(null);

  // Detail pager (paginated memories of the selected namespace).
  let detailPager = $state(createPager({ pageSize: 20 }));

  async function loadNamespaces() {
    loading = true;
    try {
      // Single call: namespace names + memory counts (uteke >= #527).
      const counted = await uteke.namespacesWithCounts();
      namespaces = counted
        .map((item) => ({ name: item.name, count: item.count }))
        .sort((a, b) => a.name.localeCompare(b.name));
    } catch {
      namespaces = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadNamespaces();
  });

  async function selectNs(ns: string) {
    selectedNs = ns;
    detailPager = createPager({ namespace: ns, pageSize: 20, useUteke: true });
    await detailPager.loadInitial();
  }
</script>

<div class="ns-view">
  <div class="ns-header">
    <h2>Namespaces</h2>
    <span class="count">{namespaces.length} namespaces</span>
  </div>

  {#if loading}
    <div class="msg">Loading...</div>
  {:else}
    <div class="layout">
      <div class="ns-list">
        {#each namespaces as ns (ns.name)}
          <button
            class="ns-card"
            class:active={selectedNs === ns.name}
            onclick={() => selectNs(ns.name)}
          >
            <div class="ns-name">{ns.name}</div>
            <span class="count">{ns.count}</span>
          </button>
        {/each}
      </div>

      <div class="ns-detail">
        {#if !selectedNs}
          <div class="msg">
            <p>Select a namespace to browse its memories</p>
          </div>
        {:else}
          <h3>{selectedNs}</h3>
          <div class="mem-list">
            {#each detailPager.items as m (m.id)}
              <div
                class="mem-card"
                role="button"
                tabindex="0"
                onclick={() => onmemoryclick(m.id)}
                onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
              >
                <div class="mem-content">{m.content.slice(0, 120)}</div>
                <div class="mem-tags">
                  {#each m.tags.slice(0, 3) as tag}<span class="tag">{tag}</span>{/each}
                </div>
              </div>
            {/each}
          </div>
          {#if detailPager.loading && detailPager.items.length === 0}
            <div class="msg">Loading…</div>
          {/if}
          {#if detailPager.hasMore}
            <div class="load-more">
              <button onclick={() => detailPager.loadMore()} disabled={detailPager.loading}>
                {detailPager.loading ? 'Loading…' : 'Load more'}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ns-view {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .ns-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 24px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .ns-header h2 { font-size: 1.1rem; margin: 0; flex: 1; }
  .ns-header .count { font-size: 0.8rem; color: var(--text-muted); }
  .layout {
    flex: 1;
    display: flex;
    gap: 0;
    overflow: hidden;
    min-height: 0;
  }
  .ns-list {
    width: 260px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    padding: 8px 12px;
    border-right: 1px solid var(--border);
  }
  .ns-card {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; text-align: left; cursor: pointer;
    background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px;
    color: var(--text-secondary); font-size: 0.85rem;
  }
  .ns-card:hover { border-color: var(--accent); }
  .ns-card.active { border-color: var(--accent); color: var(--accent); }
  .ns-name { font-weight: 500; }
  .count { color: var(--text-muted); font-size: 0.75rem; }

  .ns-detail {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 16px 24px;
  }
  .ns-detail h3 { font-size: 1rem; margin: 0 0 12px; }
  .mem-list { display: flex; flex-direction: column; gap: 8px; }
  .mem-card {
    padding: 10px 14px; cursor: pointer;
    background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px;
  }
  .mem-card:hover { border-color: var(--accent); }
  .mem-content { font-size: 0.85rem; color: var(--text-primary); margin-bottom: 6px; }
  .mem-tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.7rem; padding: 2px 6px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 3px; }

  .load-more { display: flex; justify-content: center; margin-top: 16px; }
  .load-more button {
    padding: 8px 20px; cursor: pointer; font-size: 0.85rem;
    background: var(--bg-tertiary); color: var(--text-secondary);
    border: 1px solid var(--border); border-radius: 6px;
  }
  .load-more button:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }
  .load-more button:disabled { opacity: 0.6; cursor: not-allowed; }

  .msg { text-align: center; padding: 40px; color: var(--text-muted); }
</style>
