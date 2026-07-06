<script lang="ts">
  import { uteke, system, memory as memoryApi } from '../ts/ipc';
  import { getNamespaces, invalidateNamespaces } from '../stores/cache.svelte';
  import { createPager } from '../stores/pagination.svelte';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    onmemoryclick: (id: string) => void;
  }

  let { onmemoryclick }: Props = $props();

  let namespaces = $state<{ name: string; count: number | null; source: string }[]>([]);
  let loading = $state(true);
  let selectedNs = $state<string | null>(null);

  // Detail pager (paginated memories of the selected namespace).
  let detailPager = $state(createPager({ pageSize: 20 }));

  async function loadNamespaces() {
    loading = true;
    try {
      const hubNs = await system.listNamespaces().catch(() => []);
      const utekeOk = await uteke.available().catch(() => false);

      // Fetch uteke namespace counts in a single call (uteke >= #527).
      let utekeCounts: Map<string, number> = new Map();
      if (utekeOk) {
        try {
          const counted = await uteke.namespacesWithCounts();
          for (const item of counted) utekeCounts.set(item.name, item.count);
        } catch {
          // Fallback: no counts available on this server version.
        }
      }

      const result: { name: string; count: number | null; source: string }[] = [];
      for (const ns of hubNs) result.push({ name: ns, count: null, source: 'hub' });

      if (utekeOk) {
        const utekeNs = await getNamespaces().catch(() => []);
        for (const ns of utekeNs) {
          const existing = result.find((r) => r.name === ns);
          if (existing) existing.source = 'both';
          else result.push({ name: ns, count: utekeCounts.get(ns) ?? null, source: 'uteke' });
          // Also backfill count for hub entries that match.
          const hubMatch = result.find((r) => r.name === ns);
          if (hubMatch && hubMatch.source === 'hub') hubMatch.count = utekeCounts.get(ns) ?? null;
        }
        // Backfill counts for 'both' entries.
        for (const r of result) {
          if (r.source === 'both' && r.count === null) {
            r.count = utekeCounts.get(r.name) ?? null;
          }
        }
      }
      namespaces = result.sort((a, b) => a.name.localeCompare(b.name));
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
    const utekeOk = await uteke.available().catch(() => false);
    detailPager = createPager({ namespace: ns, pageSize: 20, useUteke: utekeOk });
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
            <div class="ns-meta">
              <span class="count">{ns.count === null ? '—' : String(ns.count)}</span>
              <span class="src src-{ns.source}">{ns.source}</span>
            </div>
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
  .ns-view { padding: 16px 24px; max-width: 1000px; margin: 0 auto; }
  .ns-header { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; }
  .ns-header h2 { font-size: 1.1rem; margin: 0; flex: 1; }
  .ns-header .count { font-size: 0.8rem; color: var(--text-muted); }
  .layout { display: flex; gap: 16px; }
  .ns-list { width: 240px; display: flex; flex-direction: column; gap: 4px; max-height: 70vh; overflow-y: auto; }
  .ns-card {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; text-align: left; cursor: pointer;
    background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px;
    color: var(--text-secondary); font-size: 0.85rem;
  }
  .ns-card:hover { border-color: var(--accent); }
  .ns-card.active { border-color: var(--accent); color: var(--accent); }
  .ns-name { font-weight: 500; }
  .ns-meta { display: flex; align-items: center; gap: 6px; font-size: 0.75rem; }
  .ns-meta .count { color: var(--text-muted); }
  .src { padding: 1px 6px; border-radius: 3px; font-size: 0.65rem; text-transform: uppercase; }
  .src-uteke { background: rgba(148,226,213,0.15); color: var(--teal); }
  .src-hub { background: var(--bg-hover); color: var(--text-muted); }
  .src-both { background: rgba(245,194,231,0.15); color: var(--pink); }

  .ns-detail { flex: 1; min-width: 0; }
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
