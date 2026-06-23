<script lang="ts">
  import { onMount } from 'svelte';
  import { memory as memoryApi, uteke, utekeServer } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onnewmemory: () => void;
  }

  let { namespace, onmemoryclick, onnewmemory }: Props = $props();

  let memories = $state<(MemoryEntry & { score?: number })[]>([]);
  let loading = $state(true);
  let searchQuery = $state('');
  let activeTag = $state<string | null>(null);
  let offset = $state(0);
  let utekeReady = $state(false);
  let useSemantic = $state(false);
  const limit = 20;

  async function loadMemories() {
    loading = true;
    try {
      utekeReady = await uteke.available();

      // Check if semantic search is available
      try {
        const status = await utekeServer.status();
        useSemantic = status.available;
      } catch {
        useSemantic = false;
      }

      if (searchQuery.trim()) {
        if (useSemantic) {
          // Semantic search (default) — vector + FTS5 hybrid, top 5 only
          // Workaround for uteke #448: recall without namespace only
          // searches 'default'. Query each namespace and merge results.
          let results;
          if (namespace) {
            results = await utekeServer.recall(searchQuery, {
              namespace,
              limit: 5,
            });
          } else {
            // Fetch all namespaces and query each
            try {
              const namespaces = await uteke.namespaces();
              const allResults = await Promise.all(
                namespaces.map((ns) =>
                  utekeServer.recall(searchQuery, { namespace: ns, limit: 5 })
                )
              );
              // Flatten, sort by score, take top 5
              results = allResults
                .flat()
                .sort((a, b) => b.score - a.score)
                .slice(0, 5);
            } catch {
              results = await utekeServer.recall(searchQuery, { limit: 5 });
            }
          }
          memories = results.map((r) => ({
            id: r.id,
            content: r.content,
            tags: r.tags,
            content_type: 'text',
            importance: r.importance ?? null,
            namespace: namespace,
            created_at: null,
            updated_at: null,
            score: r.score,
          }));
        } else if (utekeReady) {
          const results = await uteke.search(searchQuery, {
            namespace: namespace ?? undefined,
            limit,
          });
          memories = results.map((r) => ({
            id: r.id,
            content: r.content,
            tags: r.tags,
            content_type: 'text',
            importance: null,
            namespace: namespace,
            created_at: null,
            updated_at: null,
          }));
        } else {
          const results = await memoryApi.search(searchQuery, { namespace: namespace ?? undefined, limit });
          memories = results.map((r) => ({
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
      } else {
        // No search query — list recent memories
        if (utekeReady) {
          if (namespace) {
            // Specific namespace selected
            memories = await uteke.list({
              namespace,
              tag: activeTag ?? undefined,
              limit,
              offset,
            });
          } else {
            // No namespace selected — fetch from ALL namespaces
            // since uteke defaults to "default" which is often empty.
            try {
              const namespaces = await uteke.namespaces();
              const allMemories = await Promise.all(
                namespaces.map((ns) =>
                  uteke.list({ namespace: ns, tag: activeTag ?? undefined, limit: Math.ceil(limit / namespaces.length) + 5 })
                )
              );
              memories = allMemories
                .flat()
                .sort((a, b) => (b.created_at ?? '').localeCompare(a.created_at ?? ''))
                .slice(offset, offset + limit);
            } catch {
              memories = await uteke.list({
                tag: activeTag ?? undefined,
                limit,
                offset,
              });
            }
          }
        } else {
          memories = await memoryApi.list({
            namespace: namespace ?? undefined,
            tag: activeTag ?? undefined,
            limit,
            offset,
          });
        }
      }
    } catch {
      memories = [];
    } finally {
      loading = false;
    }
  }

  onMount(loadMemories);

  $effect(() => {
    // Reload when namespace or filters change
    namespace;
    activeTag;
    offset = 0;
    loadMemories();
  });

  function handleSearch() {
    offset = 0;
    loadMemories();
  }
</script>

<div class="memory-list-view">
  <div class="toolbar">
    <div class="search-bar">
      <input
        type="text"
        placeholder={useSemantic ? 'Semantic search...' : 'Search memories...'}
        value={searchQuery}
        oninput={(e) => (searchQuery = e.currentTarget.value)}
        onkeydown={(e) => e.key === 'Enter' && handleSearch()}
      />
      {#if searchQuery}
        <button
          class="clear-btn"
          onclick={() => {
            searchQuery = '';
            loadMemories();
          }}>✕</button
        >
      {/if}
    </div>
    <button class="new-btn" onclick={onnewmemory}>+ New</button>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if memories.length === 0}
    <div class="empty-state">
      <p>No memories found.</p>
      <button class="new-btn" onclick={onnewmemory}>Create your first memory</button>
    </div>
  {:else}
    {#if searchQuery.trim() && useSemantic}
      <div class="search-info">Semantic search — top {memories.length} match{memories.length > 1 ? 'es' : ''}</div>
    {/if}
    <div class="list">
      {#each memories as m}
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

    <div class="pagination">
      <button disabled={offset === 0} onclick={() => { offset = Math.max(0, offset - limit); loadMemories(); }}>
        ← Prev
      </button>
      <span class="page-info">Showing {offset + 1}–{offset + memories.length}</span>
      <button
        disabled={memories.length < limit}
        onclick={() => { offset += limit; loadMemories(); }}>Next →</button
      >
    </div>
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

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-top: 16px;
  }

  .pagination button {
    padding: 6px 12px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }

  .pagination button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .pagination button:not(:disabled):hover {
    border-color: var(--accent);
  }

  .page-info {
    font-size: 0.8rem;
    color: var(--text-muted);
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
