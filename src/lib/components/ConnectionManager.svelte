<script lang="ts">
  import { onMount } from 'svelte';
  import { connection, type ConnectionInfo, type HealthInfo } from '../ts/ipc';

  let connections: ConnectionInfo[] = $state([]);
  let loading = $state(true);
  let testing = $state<string | null>(null);
  let healthResults: Record<string, HealthInfo> = $state({});

  // Add form state
  let showAdd = $state(false);
  let addName = $state('');
  let addUrl = $state('');
  let addToken = $state('');
  let addError = $state('');

  async function loadConnections() {
    loading = true;
    try {
      connections = await connection.list();
    } catch (e) {
      console.error('Failed to load connections:', e);
    } finally {
      loading = false;
    }
  }

  async function testConn(id: string) {
    testing = id;
    try {
      const result = await connection.test(id);
      healthResults = { ...healthResults, [id]: result };
      await loadConnections(); // refresh status
    } catch (e) {
      healthResults = { ...healthResults, [id]: { success: false, latency_ms: 0, version: null, error: String(e) } };
    } finally {
      testing = null;
    }
  }

  async function setPrimary(id: string) {
    try {
      await connection.setPrimary(id);
      await loadConnections();
    } catch (e) {
      console.error('Failed to set primary:', e);
    }
  }

  async function deleteConn(id: string) {
    if (!confirm('Delete this connection? This cannot be undone.')) return;
    try {
      await connection.delete(id);
      await loadConnections();
    } catch (e) {
      console.error('Failed to delete connection:', e);
    }
  }

  async function addConn() {
    addError = '';
    if (!addName.trim()) { addError = 'Name is required'; return; }
    if (!addUrl.trim()) { addError = 'URL is required'; return; }
    try {
      await connection.add({
        name: addName.trim(),
        product_type: 'uteke',
        url: addUrl.trim(),
        auth_token: addToken.trim() || undefined,
        auth_type: addToken.trim() ? 'bearer' : undefined,
      });
      addName = '';
      addUrl = '';
      addToken = '';
      showAdd = false;
      await loadConnections();
    } catch (e) {
      addError = String(e);
    }
  }

  function statusColor(status: string): string {
    switch (status) {
      case 'connected': return '#4caf50';
      case 'error': return '#f44336';
      default: return '#ff9800';
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case 'connected': return 'Connected';
      case 'error': return 'Error';
      case 'unknown': return 'Unknown';
      default: return status;
    }
  }

  function productIcon(type: string): string {
    switch (type) {
      case 'uteke': return '🔮';
      default: return '📦';
    }
  }

  onMount(loadConnections);
</script>

<div class="connection-manager">
  <div class="cm-header">
    <h3>Connections</h3>
    <button class="btn-add" onclick={() => showAdd = !showAdd}>
      {showAdd ? '✕ Cancel' : '+ Add Connection'}
    </button>
  </div>

  {#if showAdd}
    <div class="add-form card">
      <h4>New Connection</h4>
      {#if addError}
        <div class="error-banner">{addError}</div>
      {/if}
      <label>
        Name
        <input type="text" bind:value={addName} placeholder="My Uteke VPS" />
      </label>
      <label>
        Server URL
        <input type="text" bind:value={addUrl} placeholder="https://uteke.myvps.com:8767" />
      </label>
      <label>
        Auth Token <span class="optional">(optional)</span>
        <input type="password" bind:value={addToken} placeholder="Bearer token for authenticated endpoints" />
      </label>
      <div class="form-actions">
        <button class="btn-primary" onclick={addConn}>Add Connection</button>
      </div>
    </div>
  {/if}

  {#if loading}
    <p class="loading">Loading connections…</p>
  {:else if connections.length === 0}
    <p class="empty">No connections configured.</p>
  {:else}
    <div class="connection-list">
      {#each connections as conn (conn.id)}
        <div class="connection-card card" class:primary={conn.is_primary}>
          <div class="card-header">
            <span class="icon">{productIcon(conn.product_type)}</span>
            <div class="info">
              <h4>{conn.name}</h4>
              <span class="url">{conn.url}</span>
            </div>
            <span class="status-badge" style="color: {statusColor(conn.status)}">
              ● {statusLabel(conn.status)}
            </span>
          </div>

          {#if conn.is_primary}
            <span class="badge-primary">⭐ Primary</span>
          {/if}
          {#if conn.has_token}
            <span class="badge-token">🔒 Auth</span>
          {/if}

          {#if healthResults[conn.id]}
            {@const h = healthResults[conn.id]}
            <div class="health-info" class:success={h.success} class:fail={!h.success}>
              {#if h.success}
                <span>✓ Healthy — {h.latency_ms}ms</span>
              {:else}
                <span>✗ {h.error || 'Connection failed'}</span>
              {/if}
            </div>
          {/if}

          <div class="card-actions">
            <button
              class="btn-sm"
              onclick={() => testConn(conn.id)}
              disabled={testing === conn.id}
            >
              {testing === conn.id ? 'Testing…' : 'Test'}
            </button>
            {#if !conn.is_primary}
              <button class="btn-sm" onclick={() => setPrimary(conn.id)}>
                Set Primary
              </button>
            {/if}
            <button class="btn-sm btn-danger" onclick={() => deleteConn(conn.id)}>
              Delete
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .connection-manager {
    padding: 8px 0;
  }
  .cm-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }
  .cm-header h3 {
    margin: 0;
    font-size: 1.1rem;
  }
  .btn-add, .btn-primary {
    padding: 6px 14px;
    border-radius: 6px;
    border: 1px solid var(--accent, #6c5ce7);
    background: var(--accent, #6c5ce7);
    color: #fff;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .btn-add:hover, .btn-primary:hover {
    opacity: 0.9;
  }
  .card {
    background: var(--card-bg, #1a1a2e);
    border: 1px solid var(--border, #2d2d44);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
  }
  .add-form label {
    display: block;
    margin-bottom: 12px;
    font-size: 0.85rem;
    color: var(--muted, #888);
  }
  .add-form input {
    width: 100%;
    margin-top: 4px;
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--border, #2d2d44);
    background: var(--input-bg, #0f0f23);
    color: var(--fg, #e0e0e0);
    font-size: 0.9rem;
    box-sizing: border-box;
  }
  .optional {
    font-size: 0.75rem;
    opacity: 0.6;
  }
  .error-banner {
    background: #3e1a1a;
    border: 1px solid #6b2c2c;
    border-radius: 6px;
    padding: 8px 12px;
    margin-bottom: 12px;
    font-size: 0.85rem;
    color: #f44336;
  }
  .form-actions {
    margin-top: 8px;
    text-align: right;
  }
  .connection-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .connection-card.primary {
    border-color: var(--accent, #6c5ce7);
  }
  .card-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .icon {
    font-size: 1.4rem;
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .info h4 {
    margin: 0;
    font-size: 0.95rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .url {
    font-size: 0.8rem;
    color: var(--muted, #888);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status-badge {
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .badge-primary, .badge-token {
    display: inline-block;
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    margin: 4px 4px 0 0;
    background: var(--bg, #0f0f23);
    color: var(--fg, #e0e0e0);
  }
  .health-info {
    margin-top: 8px;
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 0.85rem;
  }
  .health-info.success {
    background: #1a3e1a;
    border: 1px solid #2c6b2c;
  }
  .health-info.fail {
    background: #3e1a1a;
    border: 1px solid #6b2c2c;
  }
  .card-actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .btn-sm {
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid var(--border, #2d2d44);
    background: transparent;
    color: var(--fg, #e0e0e0);
    cursor: pointer;
    font-size: 0.8rem;
  }
  .btn-sm:hover {
    background: var(--border, #2d2d44);
  }
  .btn-danger {
    color: #f44336;
    border-color: #6b2c2c;
  }
  .loading, .empty {
    color: var(--muted, #888);
    font-size: 0.9rem;
    text-align: center;
    padding: 24px;
  }
</style>
