<script lang="ts">
  import { system, memory as memoryApi, uteke, utekeServer } from '../ts/ipc';
  import type { StatsResponse, MemoryEntry } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onquicksearch: (query: string) => void;
  }

  let { namespace, onmemoryclick, onquicksearch }: Props = $props();

  let stats = $state<StatsResponse | null>(null);
  let recent = $state<MemoryEntry[]>([]);
  let searchQuery = $state('');
  let loading = $state(true);
  let utekeReady = $state(false);
  let utekeStats = $state<StatsResponse | null>(null);
  let serverOnline = $state(false);

  async function loadData() {
    loading = true;
    try {
      // Check if Uteke is available and merge data
      utekeReady = await uteke.available();

      // Check if uteke-serve is running (semantic search)
      const status = await utekeServer.status();
      serverOnline = status.available;

      if (utekeReady) {
        // Read from Uteke DB (has actual data)
        utekeStats = await uteke.stats();
        recent = await uteke.list({ namespace: namespace ?? undefined, limit: 10 });
        // Also get Hub stats for local data
        try {
          stats = await system.stats();
        } catch {
          stats = utekeStats;
        }
      } else {
        // Fallback to Hub DB only
        stats = await system.stats();
        recent = await memoryApi.list({ namespace: namespace ?? undefined, limit: 10 });
      }
    } catch {
      // store not initialized yet
    } finally {
      loading = false;
    }
  }

  // Load on mount + reload when namespace changes
  $effect(() => {
    namespace;
    loadData();
  });

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  }
</script>

<div class="dashboard">
  <div class="quick-search">
    <input
      type="text"
      placeholder={serverOnline ? 'Semantic search...' : 'Search memories...'}
      value={searchQuery}
      oninput={(e) => (searchQuery = e.currentTarget.value)}
      onkeydown={(e) => {
        if (e.key === 'Enter' && searchQuery.trim()) onquicksearch(searchQuery.trim());
      }}
    />
    <button onclick={() => searchQuery.trim() && onquicksearch(searchQuery.trim())}>Search</button>
  </div>

  {#if serverOnline}
    <div class="server-badge">
      <span class="pulse"></span>
      Uteke Server — semantic search active
    </div>
  {:else if utekeReady}
    <div class="server-badge offline">
      Uteke DB connected — run <kbd>uteke-serve</kbd> for semantic search
    </div>
  {/if}

  {#if loading}
    <div class="loading">Loading...</div>
  {:else}
    <div class="stats-grid">
      {#if utekeReady && utekeStats}
        <div class="stat-card uteke-badge">
          <div class="stat-value">{utekeStats.total_memories}</div>
          <div class="stat-label">Uteke Memories</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{stats?.total_memories ?? 0}</div>
          <div class="stat-label">Hub Memories</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{Math.max(utekeStats?.total_namespaces ?? 0, stats?.total_namespaces ?? 0)}</div>
          <div class="stat-label">Namespaces</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{Math.max(utekeStats?.total_tags ?? 0, stats?.total_tags ?? 0)}</div>
          <div class="stat-label">Tags</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{formatBytes((utekeStats?.db_size_bytes ?? 0) + (stats?.db_size_bytes ?? 0))}</div>
          <div class="stat-label">Total DB Size</div>
        </div>
      {:else}
        <div class="stat-card">
          <div class="stat-value">{stats?.total_memories ?? 0}</div>
          <div class="stat-label">Memories</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{stats?.total_namespaces ?? 0}</div>
          <div class="stat-label">Namespaces</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{stats?.total_tags ?? 0}</div>
          <div class="stat-label">Tags</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{stats?.total_edges ?? 0}</div>
          <div class="stat-label">Edges</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{formatBytes(stats?.db_size_bytes ?? 0)}</div>
          <div class="stat-label">DB Size</div>
        </div>
      {/if}
    </div>

    <div class="recent-section">
      <h2>Recent Memories</h2>
      {#if recent.length === 0}
        <div class="empty-state">
          <p>No memories yet. Create your first memory with <kbd>Ctrl+N</kbd></p>
        </div>
      {:else}
        <div class="recent-list">
          {#each recent as m}
            <div
              class="memory-item"
              role="button"
              tabindex="0"
              onclick={() => onmemoryclick(m.id)}
              onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
            >
              <div class="memory-content">{m.content.slice(0, 120)}</div>
              <div class="memory-meta">
                {#each m.tags.slice(0, 3) as tag}
                  <span class="tag">{tag}</span>
                {/each}
                {#if m.namespace}
                  <span class="namespace">{m.namespace}</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dashboard {
    padding: 24px;
    max-width: 900px;
    margin: 0 auto;
  }

  .quick-search {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .quick-search input {
    flex: 1;
    padding: 10px 14px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.95rem;
    outline: none;
  }

  .quick-search input:focus {
    border-color: var(--accent);
  }

  .quick-search button {
    padding: 10px 20px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
  }

  .quick-search button:hover {
    opacity: 0.85;
  }

  .server-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8rem;
    color: var(--teal);
    margin-bottom: 20px;
    padding: 6px 12px;
    background: rgba(137, 180, 250, 0.08);
    border-radius: 6px;
    border: 1px solid rgba(137, 180, 250, 0.2);
  }

  .server-badge.offline {
    color: var(--text-muted);
    background: var(--bg-tertiary);
    border-color: var(--border);
  }

  .pulse {
    width: 8px;
    height: 8px;
    background: var(--teal);
    border-radius: 50%;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 12px;
    margin-bottom: 32px;
  }

  .stat-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    text-align: center;
  }

  .stat-value {
    font-size: 1.8rem;
    font-weight: 700;
    color: var(--accent);
  }

  .stat-label {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .uteke-badge {
    border-color: var(--teal);
  }

  .uteke-badge .stat-value {
    color: var(--teal);
  }

  .recent-section h2 {
    font-size: 1.1rem;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .memory-item {
    padding: 12px 16px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.1s;
  }

  .memory-item:hover {
    border-color: var(--accent);
  }

  .memory-content {
    font-size: 0.9rem;
    color: var(--text-primary);
    margin-bottom: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .memory-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .tag {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 3px;
  }

  .namespace {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: rgba(137, 180, 250, 0.15);
    color: var(--accent);
    border-radius: 3px;
  }

  .empty-state {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }

  .loading {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }

  kbd {
    padding: 2px 6px;
    background: var(--bg-hover);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
</style>
