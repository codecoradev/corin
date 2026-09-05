<script lang="ts">
  import { onDestroy } from 'svelte';
  import { memory as memoryApi, uteke, utekeServer } from '../ts/ipc';
  import { createPager } from '../stores/pagination.svelte';
  import { invalidateAll } from '../stores/cache.svelte';
  import type { MemoryEntry, UnifiedSearchResult } from '../ts/types';
  import NamespaceFilter from './NamespaceFilter.svelte';
  import { FileText, Brain, X } from 'lucide-svelte';
  import { Spinner, EmptyState, Button } from '../ui';
  import { relativeTime } from '../utils/format';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onnewmemory: () => void;
    /** Open a document by slug (from unified-search document hits). */
    ondocumentclick: (slug: string) => void;
  }

  let { namespace, onmemoryclick, onnewmemory, ondocumentclick }: Props = $props();

  // ── Memories hub grouping (#293): Agents | Rooms | Tags ────────────────
  type HubGroup = 'agents' | 'rooms' | 'tags';
  let hubGroup = $state<HubGroup>('agents');
  let selectedAuthor = $state<string | null>(null);
  let selectedRoom = $state<string | null>(null);
  let selectedTag = $state<string | null>(null);

  const AUTHOR_COLORS = ['#7CB2FF', '#C4A7FF', '#4FD8D2', '#E89B3C', '#34D399', '#F87171', '#A78BFA', '#FBBF24'];

  /** Stable per-agent color: hash name -> palette index. Deterministic across sessions. */
  function authorColor(name: string): string {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
    return AUTHOR_COLORS[h % AUTHOR_COLORS.length];
  }

  function authorInitial(name: string): string {
    return (name.trim()[0] || '?').toUpperCase();
  }

  /** Provenance is stored in metadata.author (verified v0.16.0 round-trip). */
  function memoryAuthor(m: MemoryEntry & { score?: number }): string | null {
    const meta = (m as { metadata?: Record<string, unknown> }).metadata;
    const a = meta?.author;
    return typeof a === 'string' && a.trim() ? a.trim() : null;
  }

  /** Aggregate authors over everything currently loaded in the pager. */
  let authorCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const m of pager.items) {
      const a = memoryAuthor(m);
      if (a) counts.set(a, (counts.get(a) ?? 0) + 1);
    }
    return [...counts.entries()].sort((x, y) => y[1] - x[1]);
  });

  let tagCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const m of pager.items) for (const t of m.tags) counts.set(t, (counts.get(t) ?? 0) + 1);
    return [...counts.entries()].sort((x, y) => y[1] - x[1]).slice(0, 12);
  });

  let rooms = $state<{ id: string; title: string }[]>([]);
  $effect(() => {
    uteke.rooms().then((rs) => {
      rooms = (rs as { id?: string; title?: string }[]).map((r) => ({ id: String(r.id ?? ''), title: String(r.title ?? r.id ?? '') }));
    }).catch(() => { rooms = []; });
  });

  /** Client-side author/tag filter over loaded page items (server-side scope = #1181 follow-up). */
  let filteredList = $derived.by(() => {
    let items = list;
    if (selectedAuthor) items = items.filter((m) => memoryAuthor(m) === selectedAuthor);
    if (selectedTag) items = items.filter((m) => m.tags.includes(selectedTag!));
    return items;
  });

  // Cosine similarity can exceed 1; clamp so the badge never reads ">100%".
  function scorePct(score: number): number {
    return Math.min(100, Math.max(0, Math.round(score * 100)));
  }

  // Multi-namespace filter. `null` = all (show every namespace),
  // `[]` = none, array = explicit. Takes precedence over the single
  // `namespace` prop when not null.
  let selectedNamespaces = $state<string[] | null>(null);

  // Search result state (separate from paged list).
  let searchResults = $state<(MemoryEntry & { score?: number })[] | null>(null);
  let searchQuery = $state('');
  let searching = $state(false);

  // Search scope: 'memories' (memory-only recall) or 'all' (memories +
  // documents via uteke 0.9.0 unified recall — recallUnified).
  let searchMode = $state<'memories' | 'all'>('memories');
  let unifiedResults = $state<UnifiedSearchResult[] | null>(null);

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
      unifiedResults = null;
      return;
    }
    searching = true;
    try {
      await checkReady();
      // Unified search across memories + documents (uteke 0.9.0+).
      if (searchMode === 'all') {
        try {
          unifiedResults = await utekeServer.recallUnified(searchQuery, {
            searchType: 'all',
            namespace: searchNs ?? undefined,
            limit: 20,
          });
        } catch {
          // uteke < 0.9.0 (gated in the backend) or server error.
          unifiedResults = [];
        }
        searchResults = null;
        return;
      }
      unifiedResults = null;
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

  // Debounced typeahead: fire search 350ms after the user stops typing.
  // Enter still triggers an immediate search. Min length 2 to avoid
  // hammering the semantic backend on single-character keystrokes.
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleSearch(query: string) {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (query.trim().length < 2) {
      // Too short — clear any stale results so the paged list returns.
      searchResults = null;
      unifiedResults = null;
      return;
    }
    debounceTimer = setTimeout(() => runSearch(), 350);
  }
  onDestroy(() => { if (debounceTimer) clearTimeout(debounceTimer); });

  // Reload list when namespace changes; clear any active search.
  $effect(() => {
    namespace;
    selectedNamespaces;
    searchResults = null;
    unifiedResults = null;
    searchQuery = '';
    loadList();
  });

  const list = $derived<(MemoryEntry & { score?: number })[]>(
    (searchResults ?? pager.items) as (MemoryEntry & { score?: number })[]
  );
  const isLoading = $derived(searching || pager.loading);
</script>

<div class="memory-list-view">
  <aside class="hub-panel">
    <div class="hub-seg" role="group" aria-label="Group memories by">
      <button class:on={hubGroup === 'agents'} onclick={() => (hubGroup = 'agents')}>Agents</button>
      <button class:on={hubGroup === 'rooms'} onclick={() => (hubGroup = 'rooms')}>Rooms</button>
      <button class:on={hubGroup === 'tags'} onclick={() => (hubGroup = 'tags')}>Tags</button>
    </div>

    {#if hubGroup === 'agents'}
      <div class="hub-group-label">Agents <span class="hub-n">{authorCounts.length}</span></div>
      {#each authorCounts as [name, count] (name)}
        <button
          class="hub-item"
          class:on={selectedAuthor === name}
          onclick={() => (selectedAuthor = selectedAuthor === name ? null : name)}
        >
          <span class="hub-avatar" style="background: {authorColor(name)}">{authorInitial(name)}</span>
          <span class="hub-name">{name}</span>
          <span class="hub-cnt">{count}</span>
        </button>
      {:else}
        <div class="hub-empty">No authored memories on this page.</div>
      {/each}
    {:else if hubGroup === 'rooms'}
      <div class="hub-group-label">Rooms <span class="hub-n">{rooms.length}</span></div>
      {#each rooms as room (room.id)}
        <button
          class="hub-item"
          class:on={selectedRoom === room.id}
          onclick={() => (selectedRoom = selectedRoom === room.id ? null : room.id)}
          title="Room scoping lands with room-scoped pager (#297 follow-up)"
        >
          <span class="hub-ic">◫</span>
          <span class="hub-name">{room.title || room.id}</span>
        </button>
      {:else}
        <div class="hub-empty">No rooms yet.</div>
      {/each}
    {:else}
      <div class="hub-group-label">Tags <span class="hub-n">{tagCounts.length}</span></div>
      {#each tagCounts as [tag, count] (tag)}
        <button
          class="hub-item"
          class:on={selectedTag === tag}
          onclick={() => (selectedTag = selectedTag === tag ? null : tag)}
        >
          <span class="hub-ic">#</span>
          <span class="hub-name">{tag}</span>
          <span class="hub-cnt">{count}</span>
        </button>
      {:else}
        <div class="hub-empty">No tags on this page.</div>
      {/each}
    {/if}
  </aside>

  <div class="hub-main">
  <div class="toolbar">
    <div class="search-bar">
      <input
        type="text"
        placeholder="Search memories... (type or Enter)"
        value={searchQuery}
        oninput={(e) => {
          searchQuery = e.currentTarget.value;
          scheduleSearch(searchQuery);
        }}
        onkeydown={(e) => e.key === 'Enter' && runSearch()}
      />
      {#if searchQuery}
        <button
          class="clear-btn"
          onclick={() => {
            searchQuery = '';
            searchResults = null;
            unifiedResults = null;
            if (debounceTimer) clearTimeout(debounceTimer);
          }}><X size={13} strokeWidth={2.5} /></button
        >
      {/if}
    </div>
    <div class="search-mode" role="group" aria-label="Search scope">
      <button
        class="mode-btn"
        class:active={searchMode === 'memories'}
        onclick={() => {
          if (searchMode === 'memories') return;
          searchMode = 'memories';
          searchResults = null;
          unifiedResults = null;
          if (searchQuery.trim()) runSearch();
        }}>Memories</button
      >
      <button
        class="mode-btn"
        class:active={searchMode === 'all'}
        title="Search memories + documents (uteke 0.9.0+)"
        onclick={() => {
          if (searchMode === 'all') return;
          searchMode = 'all';
          searchResults = null;
          unifiedResults = null;
          if (searchQuery.trim()) runSearch();
        }}>All</button
      >
    </div>
    <button class="new-btn" onclick={onnewmemory}>+ New</button>
    <NamespaceFilter selected={selectedNamespaces} onchange={(ns) => (selectedNamespaces = ns)} />
  </div>

  <div class="scroll-area">
    {#if unifiedResults}
      <div class="search-info">
        Unified search — top {unifiedResults.length} (memories + documents)
      </div>
      {#if unifiedResults.length === 0}
        <div class="empty-state">
          <p>No matches in memories or documents.</p>
        </div>
      {:else}
        <div class="list">
          {#each unifiedResults as r (r.memory_id ?? r.doc_slug ?? r.content)}
            {#if r.result_type === 'document'}
              <div
                class="memory-card doc-card"
                role="button"
                tabindex="0"
                onclick={() => r.doc_slug && ondocumentclick(r.doc_slug)}
                onkeydown={(e) => e.key === 'Enter' && r.doc_slug && ondocumentclick(r.doc_slug)}
              >
                <div class="card-content">
                  <span class="type-badge doc">
                    <FileText size={10} strokeWidth={2.5} /> Doc
                  </span>
                  <strong>{r.doc_title ?? r.doc_slug}</strong>
                  {#if r.chunk_heading}
                    <span class="chunk-heading"> — {r.chunk_heading.replace(/^#+\s*/, '')}</span>
                  {/if}
                </div>
                {#if r.chunk_snippet}
                  <div class="doc-snippet">{r.chunk_snippet.slice(0, 200)}</div>
                {/if}
                <div class="semantic-score">{scorePct(r.score)}% match</div>
              </div>
            {:else}
              <div
                class="memory-card"
                role="button"
                tabindex="0"
                onclick={() => r.memory_id && onmemoryclick(r.memory_id)}
                onkeydown={(e) => e.key === 'Enter' && r.memory_id && onmemoryclick(r.memory_id)}
              >
                <div class="card-content">
                  <span class="type-badge mem">
                    <Brain size={10} strokeWidth={2.5} /> Memory
                  </span>
                  {r.content.slice(0, 200)}
                </div>
                <div class="semantic-score">{scorePct(r.score)}% match</div>
                <div class="card-meta">
                  <div class="tags">
                    {#each r.tags.slice(0, 5) as tag}<span class="tag">{tag}</span>{/each}
                  </div>
                  <div class="meta-right">
                    {#if r.namespace}<span class="namespace">{r.namespace}</span>{/if}
                  </div>
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    {:else if isLoading && list.length === 0}
    <div class="loading"><Spinner size={18} /> Loading...</div>
  {:else if list.length === 0}
    <EmptyState
      icon={Brain}
      title={searchQuery.trim() ? 'No memories matched.' : 'No memories yet.'}
      subtitle={searchQuery.trim()
        ? 'Nothing in the current view matches that query — try different keywords or clear the search.'
        : 'Save your first memory with Ctrl+N, or use the button below.'}
    >
      <Button variant="primary" size="sm" onclick={onnewmemory}>
        {searchQuery.trim() ? 'New Memory' : 'Create your first memory'}
      </Button>
    </EmptyState>
  {:else}
    {#if searchResults}
      <div class="search-info">Semantic search — top {searchResults.length} match{searchResults.length > 1 ? 'es' : ''}</div>
    {/if}
    <div class="list">
      {#each filteredList as m (m.id)}
        <div
          class="memory-card"
          role="button"
          tabindex="0"
          onclick={() => onmemoryclick(m.id)}
          onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
        >
          <div class="card-head">
            {#if memoryAuthor(m)}
              <span class="card-avatar" style="background: {authorColor(memoryAuthor(m)!)}">{authorInitial(memoryAuthor(m)!)}</span>
              <span class="card-author">{memoryAuthor(m)}</span>
            {:else}
              <span class="card-avatar anon">?</span>
              <span class="card-author muted">unknown</span>
            {/if}
            {#if m.created_at}<span class="card-time">{relativeTime(m.created_at)}</span>{/if}
          </div>
          <div class="card-content">{m.content.slice(0, 200)}</div>
          {#if m.score !== undefined}
            <div class="semantic-score">{scorePct(m.score)}% match</div>
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
  </div>
</div>

<style>
  .memory-list-view {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: row;
    gap: 16px;
    overflow: hidden;
    padding: 16px 24px;
    max-width: 1150px;
    margin: 0 auto;
  }

  /* ── Memories hub panel (#293) ─────────────────────────────────────── */
  .hub-panel {
    width: 216px;
    flex-shrink: 0;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    padding-right: 12px;
  }
  .hub-seg {
    display: flex;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 2px;
    margin-bottom: 10px;
  }
  .hub-seg button {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.72rem;
    padding: 4px 0;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .hub-seg button.on {
    background: var(--color-teal-bg);
    color: var(--accent);
    font-weight: 600;
  }
  .hub-group-label {
    display: flex;
    justify-content: space-between;
    font-size: 0.66rem;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--text-muted);
    padding: 10px 6px 4px;
  }
  .hub-n { color: var(--text-muted); }
  .hub-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 0.8rem;
    cursor: pointer;
    text-align: left;
  }
  .hub-item:hover { background: var(--bg-hover); color: var(--text-primary); }
  .hub-item.on {
    background: var(--color-teal-bg);
    color: var(--text-primary);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .hub-avatar {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--bg-primary);
    flex-shrink: 0;
  }
  .hub-ic { width: 18px; text-align: center; opacity: 0.7; flex-shrink: 0; }
  .hub-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hub-cnt { font-family: var(--font-mono); font-size: 0.68rem; color: var(--text-muted); }
  .hub-empty { color: var(--text-muted); font-size: 0.75rem; padding: 6px 8px; }

  .hub-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  /* ── Card author header (agent identity) ───────────────────────────── */
  .card-head {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 7px;
  }
  .card-avatar {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--bg-primary);
    flex-shrink: 0;
  }
  .card-avatar.anon { background: var(--surface1); color: var(--text-muted); }
  .card-author { font-size: 0.74rem; font-weight: 600; color: var(--text-primary); }
  .card-author.muted { color: var(--text-muted); font-weight: 400; }
  .card-time { margin-left: auto; font-size: 0.7rem; color: var(--text-muted); font-family: var(--font-mono); }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .search-bar {
    flex: 1;
    position: relative;
    min-width: 220px;
  }

  .search-bar input {
    width: 100%;
    padding: 8px 32px 8px 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.9rem;
    outline: none;
  }

  .search-bar input:focus {
    border-color: var(--accent);
  }

  .clear-btn {
    position: absolute;
    right: 6px;
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
    border-radius: var(--radius-md);
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
    border-radius: var(--radius-md);
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
    background: var(--color-green-bg);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
  }

  /* Keep content clear of the absolutely-positioned score badge. */
  .memory-card:has(.semantic-score) .card-content {
    padding-right: 88px;
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
    flex-wrap: wrap;
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
    border-radius: var(--radius-sm);
  }

  .meta-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .namespace {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: var(--color-blue-bg);
    color: var(--accent);
    border-radius: var(--radius-sm);
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
    border-radius: var(--radius-md);
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

  .search-mode {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .mode-btn {
    padding: 6px 10px;
    background: var(--bg-tertiary);
    color: var(--text-muted);
    border: none;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .mode-btn.active {
    background: var(--accent);
    color: var(--bg-primary);
    font-weight: 600;
  }
  .type-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    font-size: 0.7rem;
    margin-right: 6px;
    vertical-align: middle;
  }
  .type-badge.doc {
    background: var(--color-blue-bg);
    color: var(--accent);
  }
  .type-badge.mem {
    background: var(--color-green-bg);
    color: var(--green);
  }
  .doc-card .doc-snippet {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin-top: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chunk-heading {
    color: var(--text-muted);
    font-weight: 400;
  }
</style>
