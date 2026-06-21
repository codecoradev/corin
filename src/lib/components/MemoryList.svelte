<script lang="ts">
  import { onMount } from 'svelte';
  import { memory as memoryApi, uteke } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onnewmemory: () => void;
  }

  let { namespace, onmemoryclick, onnewmemory }: Props = $props();

  let memories = $state<MemoryEntry[]>([]);
  let loading = $state(true);
  let searchQuery = $state('');
  let activeTag = $state<string | null>(null);
  let offset = $state(0);
  let utekeReady = $state(false);
  const limit = 20;

  async function loadMemories() {
    loading = true;
    try {
      utekeReady = await uteke.available();

      if (searchQuery.trim()) {
        if (utekeReady) {
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
          memories = await memoryApi.search(searchQuery, { namespace: namespace ?? undefined, limit });
        }
      } else {
        if (utekeReady) {
          memories = await uteke.list({
            namespace: namespace ?? undefined,
            tag: activeTag ?? undefined,
            limit,
            offset,
          });
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
        placeholder="Search memories..."
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
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
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
  }

  .memory-card:hover {
    border-color: var(--accent);
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
