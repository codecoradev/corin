<script lang="ts">
  import { memory as memoryApi, uteke, utekeServer } from '../ts/ipc';
  import { createPager } from '../stores/pagination.svelte';
  import { invalidateAll } from '../stores/cache.svelte';
  import type { MemoryEntry } from '../ts/types';
  import NamespaceFilter from './NamespaceFilter.svelte';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onnewmemory: () => void;
  }

  let { namespace, onmemoryclick, onnewmemory }: Props = $props();

  // Multi-namespace filter. `null` = all (show every namespace),
  // `[]` = none, array = explicit. Takes precedence over the single
  // `namespace` prop when not null.
  let selectedNamespaces = $state<string[] | null>(null);

  // Search result state (separate from paged list).
  let searchResults = $state<(MemoryEntry & { score?: number })[] | null>(null);
  let searchQuery = $state('');
  let searching = $state(false);

  // Resolved single-namespace scope for search: the one picked namespace
  // when exactly one is selected, else fall back to the prop. Computed via
  // derived to avoid touching `.length` on a nullable state directly.
  let searchNs = $derived(
    selectedNamespaces !== null && selectedNamespaces.length === 1
      ? selectedNamespaces[0]
      : namespace,
  );

  // Paged list (no search query).
  let utekeReady = $state(false);
  let pager = $state(createPager({ namespace, pageSize: 20 }));

  async function checkReady() {
    utekeReady = await uteke.available().catch(() => false);
  }

  async function loadList() {
    await checkReady();
    // `null` (all) → backend fans out every namespace. `[]`/array → explicit.
    pager = createPager({
      namespaces: selectedNamespaces,
      namespace,
      pageSize: 20,
      useUteke: utekeReady,
    });
    await pager.loadInitial();
  }

  async function runSearch() {
    if (!searchQuery.trim()) {
      searchResults = null;
      return;
    }
    searching = true;
    try {
      await checkReady();
      // /recall is cross-namespace (uteke #448 fixed) — ONE call, no fan-out.
      // Scope to the single selected namespace when exactly one is picked;
      // search across all when multiple/all are selected.
      const ok = await utekeServer.status().then((s) => s.available).catch(() => false);
      if (ok) {
        const results = await utekeServer.recall(searchQuery, {
          namespace: searchNs ?? undefined,
          limit: 20,
        });
        searchResults = results.map((r) => ({
          id: r.id,
          content: r.content,
          tags: r.tags,
          content_type: 'text',
          importance: r.importance ?? null,
          namespace: r.namespace ?? namespace,
          created_at: null,
          updated_at: null,
          score: r.score,
        }));
      } else if (utekeReady) {
        const results = await uteke.search(searchQuery, {
          namespace: namespace ?? undefined,
          limit: 20,
        });
        searchResults = results.map((r) => ({
          id: r.id,
          content: r.content,
          tags: r.tags,
          content_type: 'text',
          importance: null,
          namespace,
          created_at: null,
          updated_at: null,
        }));
      } else {
        const results = await memoryApi.search(searchQuery, {
          namespace: namespace ?? undefined,
          limit: 20,
        });
        searchResults = results.map((r) => ({
          id: r.id,
          content: r.content,
          tags: r.tags,
          content_type: 'text',
          importance: null,
          namespace,
          created_at: null,
          updated_at: null,
        }));
      }
    } catch {
      searchResults = [];
    } finally {
      searching = false;
    }
  }

  // Reload list when namespace changes; clear any active search.
  $effect(() => {
    namespace;
    selectedNamespaces;
    searchResults = null;
    searchQuery = '';
    loadList();
  });

  const list = $derived<(MemoryEntry & { score?: number })[]>(
    (searchResults ?? pager.items) as (MemoryEntry & { score?: number })[]
  );
  const isLoading = $derived(searching || pager.loading);
</script>

<div class="memory-list-view">
  <div class="toolbar">
    <div class="search-bar">
      <input
        type="text"
        placeholder="Search memories... (Enter)"
        value={searchQuery}
        oninput={(e) => (searchQuery = e.currentTarget.value)}
        onkeydown={(e) => e.key === 'Enter' && runSearch()}
      />
      {#if searchQuery}
        <button
          class="clear-btn"
          onclick={() => {
            searchQuery = '';
            searchResults = null;
          }}>✕</button
        >
      {/if}
    </div>
    <button class="new-btn" onclick={onnewmemory}>+ New</button>
    <NamespaceFilter selected={selectedNamespaces} onchange={(ns) => (selectedNamespaces = ns)} />
  </div>

  {#if isLoading && list.length === 0}
    <div class="loading">Loading...</div>
  {:else if list.length === 0}
    <div class="empty-state">
      <p>{searchQuery.trim() ? 'No memories matched.' : 'No memories yet.'}</p>
      <button class="new-btn" onclick={onnewmemory}>Create your first memory</button>
    </div>
  {:else}
    {#if searchResults}
      <div class="search-info">Semantic search — top {searchResults.length} match{searchResults.length > 1 ? 'es' : ''}</div>
    {/if}
    <div class="list">
      {#each list as m (m.id)}
        <div
          class="memory-card"
          role="button"
          tabindex="0"
          onclick={() => onmemoryclick(m.id)}
          onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
        >
          <div class="card-content">{m.content.slice(0, 200)}</div>
          {#if m.score !== undefined}
            <div class="semantic-score">{(m.score * 100).toFixed(0)}% match</div>
          {/if}
          <div class="card-meta">
            <div class="tags">
              {#each m.tags.slice(0, 5) as tag}
                <span class="tag">{tag}</span>
              {/each}
            </div>
            <div class="meta-right">
              {#if m.namespace}
                <span class="namespace">{m.namespace}</span>
              {/if}
              {#if m.importance !== null}
                <span class="importance" title="Importance">
                  {'★'.repeat(Math.max(1, Math.round((m.importance ?? 0) * 3)))}
                </span>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>

    {#if !searchResults && pager.hasMore}
      <div class="load-more">
        <button onclick={() => pager.loadMore()} disabled={pager.loading}>
          {pager.loading ? 'Loading…' : 'Load more'}
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .memory-list-view {
    padding: 16px 24px;
    max-width: 900px;
    margin: 0 auto;
  }

  .toolbar {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }

  .search-bar {
    flex: 1;
    position: relative;
  }

  .search-bar input {
    width: 100%;
    padding: 8px 32px 8px 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.9rem;
    outline: none;
  }

  .search-bar input:focus {
    border-color: var(--accent);
  }

  .clear-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.9rem;
    padding: 2px 6px;
  }

  .new-btn {
    padding: 8px 16px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .new-btn:hover {
    opacity: 0.85;
  }

  .search-info {
    font-size: 0.75rem;
    color: var(--green);
    padding: 4px 0 8px;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .memory-card {
    padding: 12px 16px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.1s;
    position: relative;
  }

  .memory-card:hover {
    border-color: var(--accent);
  }

  .semantic-score {
    position: absolute;
    top: 8px;
    right: 12px;
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--green);
    background: rgba(166, 227, 161, 0.12);
    padding: 2px 8px;
    border-radius: 3px;
  }

  .card-content {
    font-size: 0.9rem;
    color: var(--text-primary);
    margin-bottom: 8px;
    line-height: 1.4;
  }

  .card-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .tag {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 3px;
  }

  .meta-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .namespace {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: rgba(137, 180, 250, 0.15);
    color: var(--accent);
    border-radius: 3px;
  }

  .importance {
    font-size: 0.7rem;
    color: var(--yellow);
  }

  .load-more {
    display: flex;
    justify-content: center;
    margin-top: 16px;
  }

  .load-more button {
    padding: 8px 20px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .load-more button:not(:disabled):hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .load-more button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .loading,
  .empty-state {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }

  .empty-state button {
    margin-top: 12px;
  }
</style>
