<script lang="ts">
  import { onDestroy } from 'svelte';
  import { memory as memoryApi, uteke, utekeServer, system } from '../ts/ipc';
  import { createPager } from '../stores/pagination.svelte';
  import { invalidateAll } from '../stores/cache.svelte';
  import type { MemoryEntry, UnifiedSearchResult } from '../ts/types';
  import { FileText, Brain, X, Pin, User, Bot } from 'lucide-svelte';
  import { authorClass } from '../utils/author';
  import { Spinner, EmptyState, Button } from '../ui';
  import { relativeTime } from '../utils/format';
  import { kbdCombo } from '../utils/platform';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onnewmemory: () => void;
    /** Open a document by slug (from unified-search document hits). */
    ondocumentclick: (slug: string) => void;
    /** Open the graph view. A callback, not a location.hash hack: the hash
        never changes back in desktop mode, so a second identical #graph
        assignment fired no hashchange and the button died. */
    ongraph: () => void;
  }

  let { namespace, onmemoryclick, onnewmemory, ondocumentclick, ongraph }: Props = $props();

  // ── Memories hub: Rooms | Tags facets + destination navigation ─────────
  type HubGroup = 'namespaces' | 'rooms' | 'tags';
  let hubGroup = $state<HubGroup>('namespaces');

  /**
   * Single source of scope truth — where the user has navigated to. Sidebar
   * clicks set it; the content area (list, chips, load-more) derives from
   * it; the toolbar NamespaceFilter refines only within 'all'. One
   * destination at a time, so the controls can never fight each other.
   */
  type Destination =
    | { type: 'all' }
    | { type: 'namespace'; name: string }
    | { type: 'room'; id: string; title: string; namespace?: string }
    | { type: 'tag'; name: string };
  let destination = $state<Destination>({ type: 'all' });

  function isActive(
    d: { type: Destination['type']; id?: string; name?: string },
  ): boolean {
    if (destination.type !== d.type) return false;
    switch (destination.type) {
      case 'namespace':
        return d.name === destination.name;
      case 'room':
        return d.id === destination.id;
      case 'tag':
        return d.name === destination.name;
      default:
        return true;
    }
  }
  function goNamespace(name: string) {
    destination = { type: 'namespace', name };
  }
  function goRoom(room: { id: string; title: string; namespace?: string }) {
    destination = { type: 'room', id: room.id, title: room.title, namespace: room.namespace };
  }
  function goTag(name: string) {
    destination = { type: 'tag', name };
  }
  function goAll() {
    destination = { type: 'all' };
  }

  // Hub Namespaces state — derivations live with the hub filter below.

  /** Page-derived fallback — only shown when the server tag list is unreachable. */
  let tagCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const m of pager.items) for (const t of m.tags) counts.set(t, (counts.get(t) ?? 0) + 1);
    return [...counts.entries()]
      .sort((x, y) => y[1] - x[1])
      .slice(0, 12)
      .map(([name, count]) => ({ name, count }));
  });


  // Hub Namespaces group — server list with real counts, always populated.
  // Clicking one scopes the list to that single namespace (shared state with
  // the toolbar filter, so both always agree).
  let hubNamespaces = $state<{ name: string; count: number }[] | null>(null);
  let hubNsLoading = $state(false);

  async function loadHubNamespaces() {
    hubNsLoading = true;
    try {
      // Breakdown (uteke >= 0.16.1) carries the ACTIVE count — the same
      // number clicking the namespace will show. Fall back to totals.
      const rows = await uteke.namespacesBreakdown();
      hubNamespaces = rows
        .map((r) => ({ name: r.name, count: r.active ?? r.count ?? 0 }))
        .sort((a, b) => a.name.localeCompare(b.name));
    } catch {
      try {
        const rows = await uteke.namespacesWithCounts();
        hubNamespaces = rows
          .map((r) => ({ name: r.name, count: r.count }))
          .sort((a, b) => a.name.localeCompare(b.name));
      } catch {
        hubNamespaces = [];
      }
    } finally {
      hubNsLoading = false;
    }
  }

  let rooms = $state<{ id: string; title: string; count?: number; namespace?: string }[]>([]);
  $effect(() => {
    uteke.rooms().then((rs) => {
      rooms = (rs as { id?: string; title?: string; namespace?: string; memory_count?: number }[]).map((r) => ({
        id: String(r.id ?? ''),
        title: String(r.title ?? r.id ?? ''),
        namespace: r.namespace,
        count: (r as { memory_count?: number }).memory_count,
      }));
    }).catch(() => { rooms = []; });
  });

  // ── Room scope (#297): clicking a room loads THAT room's memories
  // (GET /room/memories with /room/recall fallback inside the command) and
  // syncs the namespace filter to the room's namespace so the toolbar agrees
  // with what's on screen. Changing the toolbar filter drops the room scope.
  let roomItems = $state<MemoryEntry[] | null>(null);
  let roomLoading = $state(false);

  async function loadRoom(id: string) {
    roomLoading = true;
    try {
      roomItems = (await uteke.roomMemories(id, { limit: 200 })) ?? [];
    } catch {
      roomItems = [];
    } finally {
      roomLoading = false;
    }
  }

  /** Pinned memories float to the top of the browsed list; search keeps its
      relevance order. */
  let filteredList = $derived.by(() => {
    let items = list;
    if (!searchResults) {
      items = [...items].sort(
        (a, b) => Number(b.pinned ?? false) - Number(a.pinned ?? false),
      );
    }
    return items;
  });

  // Cosine similarity can exceed 1; clamp so the badge never reads ">100%".
  function scorePct(score: number): number {
    return Math.min(100, Math.max(0, Math.round(score * 100)));
  }

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
  let searchNs = $derived(namespace);

  // Paged list (no search query).
  let utekeReady = $state(false);
  let pager = $state(createPager({ namespace, pageSize: 20 }));

  async function checkReady() {
    utekeReady = await uteke.available().catch(() => false);
  }

  async function loadList(d: Destination, globalNs: string | null) {
    await checkReady();
    if (d.type === 'namespace') {
      pager = createPager({ namespace: d.name, pageSize: 20, useUteke: utekeReady });
    } else if (d.type === 'tag') {
      // Tag destination: server-side scope so counts and the list agree.
      pager = createPager({ tag: d.name, pageSize: 20, useUteke: utekeReady });
    } else {
      // 'all' → backend fans out every namespace.
      pager = createPager({ namespace: globalNs, pageSize: 20, useUteke: utekeReady });
    }
    await pager.loadInitial();
  }

  // ── Tags panel: real server counts (GET /tags via list_tags) ──────────
  // The full tag list arrives in one call (uteke ≥ 0.17 also pages it via
  // continuation; 0.15 returns everything), so we window the RENDER instead:
  // TAG_WINDOW rows at a time, growing via "Load more". The header count is
  // always the true total.
  interface TagCount { name: string; count: number }
  const TAG_WINDOW = 30;
  let serverTags = $state<TagCount[] | null>(null);
  let tagsLoading = $state(false);
  let tagWindow = $state(TAG_WINDOW);

  // Hub panel filter — one query filters whichever group is active. For
  // Tags it spans the FULL server list, not the rendered window, so a tag
  // is findable without clicking Load first.
  let hubQuery = $state('');

  function hubMatches(name: string): boolean {
    const q = hubQuery.trim().toLowerCase();
    return !q || name.toLowerCase().includes(q);
  }

  let visibleHubNamespaces = $derived((hubNamespaces ?? []).filter((ns) => hubMatches(ns.name)));
  let hubNsHeader = $derived(
    hubQuery.trim() ? visibleHubNamespaces.length : (hubNamespaces?.length ?? 0),
  );

  /** Hub namespace click: navigate to that namespace's destination. */
  function selectNsFilter(name: string) {
    goNamespace(name);
  }

  let visibleRooms = $derived(rooms.filter((r) => hubMatches(r.title || r.id)));

  let visibleTags = $derived.by(() => {
    if (serverTags) {
      return hubQuery.trim()
        ? serverTags.filter((t) => hubMatches(t.name))
        : serverTags.slice(0, tagWindow);
    }
    return tagCounts.filter((t) => hubMatches(t.name));
  });
  let tagsTotal = $derived(serverTags ? serverTags.length : tagCounts.length);
  // While filtering, the header reflects the match count instead of the total.
  let tagsHeader = $derived(hubQuery.trim() ? visibleTags.length : tagsTotal);
  let hiddenTags = $derived(
    serverTags && !hubQuery.trim() ? Math.max(0, serverTags.length - tagWindow) : 0,
  );

  async function loadTags() {
    tagsLoading = true;
    tagWindow = TAG_WINDOW;
    try {
      if (!(await uteke.available().catch(() => false))) {
        serverTags = null;
        return;
      }
      const rows = await system.listTags();
      serverTags = rows.sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
    } catch {
      serverTags = null; // offline / old backend → page-derived fallback
    } finally {
      tagsLoading = false;
    }
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

  // Reload when the destination or the 'all'-mode namespace refinement
  // changes; clear any active search. Room destinations load their own
  // listing instead of the pager.
  $effect(() => {
    const d = destination;
    const globalNs = namespace;
    searchResults = null;
    unifiedResults = null;
    searchQuery = '';
    if (d.type === 'room') {
      roomLoading = true;
      uteke
        .roomMemories(d.id, { limit: 200 })
        .then((items) => {
          roomItems = items ?? [];
        })
        .catch(() => {
          roomItems = [];
        })
        .finally(() => {
          roomLoading = false;
        });
      return;
    }
    void loadList(d, globalNs);
  });

  // Hub namespace list is global — load once per view mount.
  $effect(() => {
    loadHubNamespaces();
  });

  // Standalone hub: tag counts are global, unaffected by the toolbar
  // namespace filter.
  $effect(() => {
    loadTags();
  });

  type ListItem = MemoryEntry & { score?: number };
  const list = $derived.by((): ListItem[] => {
    if (searchResults) return searchResults as ListItem[];
    if (destination.type === 'room') return roomItems ?? [];
    return pager.items as ListItem[];
  });
  const isLoading = $derived(searching || pager.loading || roomLoading);
</script>

<div class="memory-list-view">
  <aside class="hub-panel">
    <div class="hub-head">
  <div class="hub-seg" role="group" aria-label="Group memories by">
      <button class:on={hubGroup === 'namespaces'} onclick={() => (hubGroup = 'namespaces')}>Namespaces</button>
      <button class:on={hubGroup === 'rooms'} onclick={() => (hubGroup = 'rooms')}>Rooms</button>
      <button class:on={hubGroup === 'tags'} onclick={() => (hubGroup = 'tags')}>Tags</button>
    </div>

    <div class="hub-search">
      <input
        type="text"
        placeholder="Filter {hubGroup}…"
        bind:value={hubQuery}
        onkeydown={(e) => e.key === 'Escape' && (hubQuery = '')}
      />
      {#if hubQuery}
        <button class="hub-clear" onclick={() => (hubQuery = '')} aria-label="Clear filter">
          <X size={11} strokeWidth={2.5} />
        </button>
      {/if}
    </div>
    </div>

    <div class="hub-list">
    {#if hubGroup === 'namespaces'}
      <div class="hub-group-label">Namespaces <span class="hub-n">{hubNsHeader}</span></div>
      <button
        class="hub-item"
        class:on={destination.type === 'all'}
        onclick={goAll}
        title="Show memories from every namespace"
      >
        <span class="hub-ic">✦</span>
        <span class="hub-name">All namespaces</span>
      </button>
      {#each visibleHubNamespaces as ns (ns.name)}
        <button
          class="hub-item"
          class:on={isActive({ type: 'namespace', name: ns.name })}
          onclick={() => (isActive({ type: 'namespace', name: ns.name }) ? goAll() : goNamespace(ns.name))}
          title="Show memories from namespace {ns.name}"
        >
          <span class="hub-ic">◇</span>
          <span class="hub-name">{ns.name}</span>
          <span class="hub-cnt">{ns.count}</span>
        </button>
      {:else}
        <div class="hub-empty">{hubNsLoading ? 'Loading…' : 'No namespaces yet.'}</div>
      {/each}
    {:else if hubGroup === 'rooms'}
      <div class="hub-group-label">Rooms <span class="hub-n">{visibleRooms.length}</span></div>
      {#each visibleRooms as room (room.id)}
        <button
          class="hub-item"
          class:on={isActive({ type: 'room', id: room.id })}
          onclick={() => (isActive({ type: 'room', id: room.id }) ? goAll() : goRoom(room))}
          title={`Show memories from namespace ${room.namespace ?? '—'}`}
        >
          <span class="hub-ic">◫</span>
          <span class="hub-name">{room.title || room.id}</span>
          {#if room.count !== undefined}<span class="hub-cnt">{room.count}</span>{/if}
        </button>
      {:else}
        <div class="hub-empty">{hubQuery.trim() ? 'No matches.' : 'No rooms yet.'}</div>
      {/each}
    {:else}
      <div class="hub-group-label">Tags <span class="hub-n">{tagsHeader}</span></div>
      {#each visibleTags as t (t.name)}
        <button
          class="hub-item"
          class:on={isActive({ type: 'tag', name: t.name })}
          onclick={() => (isActive({ type: 'tag', name: t.name }) ? goAll() : goTag(t.name))}
          title="Show memories tagged #{t.name}"
        >
          <span class="hub-ic">#</span>
          <span class="hub-name">{t.name}</span>
          <span class="hub-cnt">{t.count}</span>
        </button>
      {:else}
        <div class="hub-empty">{tagsLoading ? 'Loading tags…' : 'No matches.'}</div>
      {/each}
      {#if hiddenTags > 0}
        <button class="hub-more" onclick={() => (tagWindow += TAG_WINDOW)}>
          Load more · {hiddenTags} more
        </button>
      {/if}
    {/if}
    </div>
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
    <button class="graph-link" title="Open graph exploration" onclick={() => ongraph()}>⌗ Graph</button>
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
      title={destination.type === 'room'
        ? 'No memories in this room yet.'
        : searchQuery.trim()
          ? 'No memories matched.'
          : 'No memories yet.'}
      subtitle={searchQuery.trim()
        ? 'Nothing in the current view matches that query — try different keywords or clear the search.'
        : `Save your first memory with ${kbdCombo('N')}, or use the button below.`}
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
            {#if authorClass(m) === 'human'}
              <span class="author-badge human" title="Written by a human"><User size={11} strokeWidth={2.25} /> Human</span>
            {:else}
              <span class="author-badge" title="Written by an agent"><Bot size={11} strokeWidth={2.25} /> Agent</span>
            {/if}
            {#if m.pinned}
              <span class="card-pin" title="Pinned"><Pin size={11} strokeWidth={2.5} /></span>
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

    {#if !searchResults && destination.type !== 'room' && pager.hasMore}
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
  }

  /* ── Memories hub panel (#293) ─────────────────────────────────────── */
  /* Hub panel: header (group switch + filter) stays fixed; only the list
     scrolls — switching facets never requires scrolling back up. */
  .hub-panel {
    width: 216px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid var(--border);
    padding: 0 12px;
  }
  .hub-head {
    flex-shrink: 0;
    padding-bottom: 8px;
  }
  .hub-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    margin: 0 -12px;
    padding: 0 12px;
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
  .hub-search { position: relative; margin-bottom: 8px; }
  .hub-search input {
    width: 100%;
    padding: 5px 24px 5px 10px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.78rem;
    outline: none;
  }
  .hub-search input:focus { border-color: var(--accent); }
  .hub-clear {
    position: absolute;
    right: 5px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
  }
  .hub-clear:hover { color: var(--text-primary); }
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
  .hub-ic { width: 18px; text-align: center; opacity: 0.7; flex-shrink: 0; }
  .hub-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hub-cnt { font-family: var(--font-mono); font-size: 0.68rem; color: var(--text-muted); }
  .hub-empty { color: var(--text-muted); font-size: 0.75rem; padding: 6px 8px; }
  .hub-more {
    display: block;
    width: calc(100% - 16px);
    margin: 6px 8px;
    padding: 5px 0;
    background: transparent;
    border: 1px dashed var(--border);
    border-radius: var(--radius-md);
    color: var(--text-muted);
    font-size: 0.72rem;
    cursor: pointer;
    text-align: center;
  }
  .hub-more:hover { color: var(--accent); border-color: var(--accent); }

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
  .author-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 7px;
    border-radius: var(--radius-pill);
    background: var(--bg-hover);
    color: var(--text-muted);
    font-size: 0.68rem;
    flex-shrink: 0;
  }
  .author-badge.human {
    background: var(--color-blue-bg);
    color: var(--accent);
  }
  .card-pin { color: var(--accent); display: inline-flex; align-items: center; flex-shrink: 0; }
  .card-time { margin-left: auto; font-size: 0.7rem; color: var(--text-muted); font-family: var(--font-mono); }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    padding-right: 12px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
    flex-wrap: wrap;
    /* Matches the cards' right edge: .scroll-area's 12px inset + the 10px
       scrollbar gutter drawn inside it. */
    padding-right: 22px;
  }

  .search-bar {
    flex: 1;
    position: relative;
    min-width: 220px;
  }

  .search-bar input {
    width: 100%;
    height: 36px;
    padding: 0 32px 0 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.88rem;
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

    .graph-link {
    height: 36px;
    padding: 0 14px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .graph-link:hover { background: var(--bg-hover); color: var(--text-primary); }

  .new-btn {
    height: 36px;
    padding: 0 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: 0.85rem;
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
    height: 36px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .mode-btn {
    padding: 0 14px;
    background: var(--bg-tertiary);
    color: var(--text-muted);
    border: none;
    font-size: 0.85rem;
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
