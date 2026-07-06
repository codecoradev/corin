<script lang="ts">
  import { system, memory as memoryApi, utekeServer, ecosystem } from '../ts/ipc';
  import { getStats } from '../stores/cache.svelte';
  import type {
    StatsResponse,
    MemoryEntry,
    ProductCard,
    ProductHealth,
  } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
    onquicksearch: (query: string) => void;
  }

  let { namespace, onmemoryclick, onquicksearch }: Props = $props();

  // ─── Product cards ───────────────────────────────────────────────
  // Static registry of CodeCora ecosystem products.
  // Each product has a default local URL and a health endpoint path.
  const products: ProductCard[] = [
    {
      id: 'uteke',
      name: 'Uteke',
      icon: '🧠',
      description: 'Memories, Graph',
      color: 'var(--teal)',
      defaultUrl: 'http://127.0.0.1:8767',
      healthPath: '/health',
    },
    {
      id: 'cora',
      name: 'Cora',
      icon: '🔍',
      description: 'Code Reviews, SARIF',
      color: 'var(--accent)',
      defaultUrl: 'http://127.0.0.1:8768',
      healthPath: '/api/health',
    },
    {
      id: 'trapfall',
      name: 'TrapFall',
      icon: '🪤',
      description: 'Error Groups, Crashes',
      color: 'var(--peach)',
      defaultUrl: 'http://127.0.0.1:8769',
      healthPath: '/api/health',
    },
    {
      id: 'rungu',
      name: 'Rungu',
      icon: '👂',
      description: 'Feedback, Code Intel',
      color: 'var(--mauve)',
      defaultUrl: 'http://127.0.0.1:8770',
      healthPath: '/api/health',
    },
  ];

  let health: Record<string, ProductHealth> = $state({});
  let healthLoading = $state<Record<string, boolean>>({});

  async function checkAllProducts() {
    const checks = products.map(async (p) => {
      healthLoading[p.id] = true;
      try {
        const result = await ecosystem.checkHealth(
          p.defaultUrl,
          p.healthPath,
        );
        health[p.id] = result;
      } catch {
        health[p.id] = {
          id: p.id,
          available: false,
          latency_ms: 0,
          version: null,
          error: 'check failed',
        };
      } finally {
        healthLoading[p.id] = false;
      }
    });
    await Promise.allSettled(checks);
  }

  // ─── Uteke stats + recent memories ─────────────────────────────────
  let stats = $state<StatsResponse | null>(null);
  let recent = $state<MemoryEntry[]>([]);
  let searchQuery = $state('');
  let loading = $state(true);
  let serverOnline = $state(false);

  async function loadData() {
    loading = true;
    try {
      const [s, status] = await Promise.all([
        getStats(),
        utekeServer.status().catch(() => ({ available: false })),
      ]);
      stats = s;
      serverOnline = status.available;

      if (serverOnline) {
        recent = await utekeServer
          .recent({ namespace, limit: 10 })
          .catch(() => []);
      } else if (namespace) {
        recent = await memoryApi
          .list({ namespace, limit: 10 })
          .catch(() => []);
      } else {
        recent = await memoryApi.list({ limit: 10 }).catch(() => []);
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
    checkAllProducts();
  });

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  }

  function statusLabel(h: ProductHealth | undefined): string {
    if (!h) return 'Checking...';
    if (h.available) return `${h.latency_ms}ms`;
    return 'Offline';
  }
</script>

<div class="dashboard">
  <!-- Product cards -->
  <section class="products-section">
    <h2 class="section-title">CodeCora Ecosystem</h2>
    <div class="products-grid">
      {#each products as product}
        <div
          class="product-card"
          class:online={health[product.id]?.available === true}
          class:offline={!health[product.id] || health[product.id]?.available === false}
        >
          <div class="product-header">
            <span class="product-icon">{product.icon}</span>
            <div class="product-info">
              <span class="product-name" style="color: {product.color}"
                >{product.name}</span
              >
              <span class="product-desc">{product.description}</span>
            </div>
            <span
              class="status-badge"
              class:badge-online={health[product.id]?.available === true}
              class:badge-offline={!health[product.id] || health[product.id]?.available === false}
            >
              {#if healthLoading[product.id]}
                ...
              {:else}
                {statusLabel(health[product.id])}
              {/if}
            </span>
          </div>
          <div class="product-meta">
            {#if health[product.id]?.version}
              <span class="version-tag">v{health[product.id].version}</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- Quick search -->
  <div class="quick-search">
    <input
      type="text"
      placeholder={serverOnline ? 'Semantic search...' : 'Search memories...'}
      value={searchQuery}
      oninput={(e) => (searchQuery = e.currentTarget.value)}
      onkeydown={(e) => {
        if (e.key === 'Enter' && searchQuery.trim())
          onquicksearch(searchQuery.trim());
      }}
    />
    <button
      onclick={() => searchQuery.trim() && onquicksearch(searchQuery.trim())}
    >
      Search
    </button>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else}
    <!-- Uteke stats -->
    <section class="stats-section">
      <h2 class="section-title">Uteke Memory</h2>
      <div class="stats-grid">
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
          <div class="stat-value">{formatBytes(stats?.db_size_bytes ?? 0)}</div>
          <div class="stat-label">DB Size</div>
        </div>
      </div>
    </section>

    <!-- Recent memories -->
    <div class="recent-section">
      <h2 class="section-title">Recent Memories</h2>
      {#if recent.length === 0}
        <div class="empty-state">
          <p>
            No memories yet. Create your first memory with
            <kbd>Ctrl+N</kbd>
          </p>
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
              <div class="memory-content">
                {m.content.slice(0, 120)}
              </div>
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
    max-width: 960px;
    margin: 0 auto;
  }

  .section-title {
    font-size: 1rem;
    color: var(--text-secondary);
    margin-bottom: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  /* ─── Product cards ─────────────────────────────────────────────── */
  .products-section {
    margin-bottom: 28px;
  }

  .products-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px;
  }

  .product-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    transition: border-color 0.15s;
  }

  .product-card.online {
    border-color: var(--green);
  }

  .product-card.offline {
    opacity: 0.6;
  }

  .product-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .product-icon {
    font-size: 1.4rem;
    flex-shrink: 0;
  }

  .product-info {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .product-name {
    font-size: 0.9rem;
    font-weight: 700;
    line-height: 1.2;
  }

  .product-desc {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .status-badge {
    font-size: 0.7rem;
    padding: 2px 8px;
    border-radius: 10px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-online {
    background: rgba(166, 227, 161, 0.15);
    color: var(--green);
  }

  .badge-offline {
    background: var(--bg-hover);
    color: var(--text-muted);
  }

  .product-meta {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .version-tag {
    font-size: 0.7rem;
    padding: 1px 6px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 3px;
  }

  /* ─── Quick search ──────────────────────────────────────────────── */
  .quick-search {
    display: flex;
    gap: 8px;
    margin-bottom: 24px;
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

  /* ─── Stats ─────────────────────────────────────────────────────── */
  .stats-section {
    margin-bottom: 28px;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 12px;
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
    color: var(--teal);
  }

  .stat-label {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 4px;
  }

  /* ─── Recent memories ───────────────────────────────────────────── */
  .recent-section {
    margin-bottom: 28px;
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
