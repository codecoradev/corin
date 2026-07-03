<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { connection, type HealthInfo } from '../ts/ipc';
  import { getConnectionsStore } from '../stores/connections.svelte';

  const store = getConnectionsStore();

  // Local UI state (test results, add form) stays component-scoped.
  let testing = $state<string | null>(null);
  let reconnecting = $state<string | null>(null);
  let disconnecting = $state(false);
  let healthResults: Record<string, HealthInfo> = $state({});

  // Delete confirmation dialog (native confirm() is blocked in Tauri webview).
  let pendingDelete = $state<{ id: string; name: string } | null>(null);

  // Add form state
  let showAdd = $state(false);
  let addName = $state('');
  let addUrl = $state('');
  let addToken = $state('');
  let addError = $state('');

  // Edit form state.  The token field is always blank on edit (the backend
  // never returns the stored token — only a `has_token` boolean).  Leaving it
  // blank sends no update for that field, so the existing token is kept.
  let editId = $state<string | null>(null);
  let editName = $state('');
  let editUrl = $state('');
  let editToken = $state('');
  let editError = $state('');
  let saving = $state(false);

  const connections = $derived(store.connections);
  const loading = $derived(store.loading);

  async function loadConnections() {
    await store.refresh();
  }

  async function testConn(id: string) {
    testing = id;
    try {
      const result = await connection.test(id);
      healthResults = { ...healthResults, [id]: result };
      await loadConnections();
    } catch (e) {
      healthResults = { ...healthResults, [id]: { success: false, latency_ms: 0, version: null, error: String(e) } };
    } finally {
      testing = null;
    }
  }

  async function reconnectConn(id: string) {
    reconnecting = id;
    try {
      const result = await store.reconnect(id);
      healthResults = { ...healthResults, [id]: result };
    } catch (e) {
      healthResults = { ...healthResults, [id]: { success: false, latency_ms: 0, version: null, error: String(e) } };
    } finally {
      reconnecting = null;
    }
  }

  async function setPrimary(id: string) {
    try {
      await store.setPrimary(id);
    } catch (e) {
      console.error('Failed to set primary:', e);
    }
  }

  async function disconnectConn() {
    disconnecting = true;
    try {
      await store.disconnect();
    } catch (e) {
      console.error('Failed to disconnect:', e);
    } finally {
      disconnecting = false;
    }
  }

  function requestDelete(id: string, name: string) {
    pendingDelete = { id, name };
  }

  function cancelDelete() {
    pendingDelete = null;
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    const id = pendingDelete.id;
    pendingDelete = null;
    try {
      await store.remove(id);
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
        productType: 'uteke',
        url: addUrl.trim(),
        authToken: addToken.trim() || undefined,
        authType: addToken.trim() ? 'bearer' : undefined,
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

  /** Open the edit form, pre-filled with the connection's name + url.
   *  The token field is intentionally left blank (security: the backend
   *  never returns the stored token). Leaving it blank keeps the existing
   *  token; typing a new value replaces it. */
  function startEdit(id: string, name: string, url: string) {
    editId = id;
    editName = name;
    editUrl = url;
    editToken = '';
    editError = '';
  }

  function cancelEdit() {
    editId = null;
    editName = '';
    editUrl = '';
    editToken = '';
    editError = '';
  }

  async function saveEdit() {
    if (!editId) return;
    editError = '';
    if (!editName.trim()) { editError = 'Name is required'; return; }
    if (!editUrl.trim()) { editError = 'URL is required'; return; }
    saving = true;
    const id = editId;
    try {
      await connection.update({
        id,
        name: editName.trim(),
        url: editUrl.trim(),
        // blank token = don't send the field → backend keeps existing token
        authToken: editToken.trim() || undefined,
        authType: editToken.trim() ? 'bearer' : undefined,
      });
      cancelEdit();
      await loadConnections();
      // If we just edited the primary connection, live-rebuild the client
      // so URL/token changes take effect immediately (no app restart).
      const updated = connections.find((c) => c.id === id);
      if (updated?.is_primary) {
        try { await store.reconnect(id); } catch { /* user can retry */ }
      }
    } catch (e) {
      editError = String(e);
    } finally {
      saving = false;
    }
  }

  function statusColor(status: string): string {
    switch (status) {
      case 'connected': return '#4caf50';
      case 'disconnected': return '#9e9e9e';
      case 'error': return '#f44336';
      default: return '#ff9800';
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case 'connected': return 'Connected';
      case 'disconnected': return 'Disconnected';
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

  onMount(() => {
    void loadConnections();
    store.startPolling();
  });

  onDestroy(() => store.stopPolling());
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
              class="btn-sm btn-icon"
              onclick={() => startEdit(conn.id, conn.name, conn.url)}
              disabled={!!editId}
              title="Edit connection"
            >✎</button>
            <button
              class="btn-sm"
              onclick={() => testConn(conn.id)}
              disabled={testing === conn.id}
            >
              {testing === conn.id ? 'Testing…' : 'Test'}
            </button>
            <button
              class="btn-sm"
              onclick={() => reconnectConn(conn.id)}
              disabled={reconnecting === conn.id}
              title="Rebuild the live backend from this connection (no restart)"
            >
              {reconnecting === conn.id ? 'Reconnecting…' : 'Reconnect'}
            </button>
            {#if conn.is_primary && conn.status === 'connected'}
              <button
                class="btn-sm"
                onclick={disconnectConn}
                disabled={disconnecting}
                title="Disconnect the active memory backend (recall/search will fail until reconnect)"
              >
                {disconnecting ? 'Disconnecting…' : 'Disconnect'}
              </button>
            {/if}
            {#if !conn.is_primary && conn.status === 'connected'}
              <button class="btn-sm" onclick={() => setPrimary(conn.id)}>
                Set Primary
              </button>
            {/if}
            <button class="btn-sm btn-danger" onclick={() => requestDelete(conn.id, conn.name)}>
              Delete
            </button>
          </div>

          {#if editId === conn.id}
            <div class="edit-form">
              <h4>Edit “{conn.name}”</h4>
              {#if editError}
                <div class="error-banner">{editError}</div>
              {/if}
              <label>
                Name
                <input type="text" bind:value={editName} placeholder="My Uteke VPS" />
              </label>
              <label>
                Server URL
                <input type="text" bind:value={editUrl} placeholder="https://uteke.myvps.com:8767" />
              </label>
              <label>
                Auth Token <span class="optional">(leave blank to keep current{conn.has_token ? ' 🔒' : ''})</span>
                <input type="password" bind:value={editToken} placeholder={conn.has_token ? '•••••••• (set new to replace)' : 'Bearer token (optional)'} />
              </label>
              <div class="form-actions">
                <button class="btn-sm" onclick={cancelEdit} disabled={saving}>Cancel</button>
                <button class="btn-primary" onclick={saveEdit} disabled={saving}>
                  {saving ? 'Saving…' : 'Save'}
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if pendingDelete}
  <div
    class="confirm-overlay"
    role="button"
    tabindex="0"
    onclick={cancelDelete}
    onkeydown={(e) => e.key === 'Escape' && cancelDelete()}
  >
    <div class="confirm-dialog" onclick={(e) => e.stopPropagation()} role="presentation">
      <h3>Delete “{pendingDelete.name}”?</h3>
      <p>The auth token will be wiped and the connection row removed. This cannot be undone.</p>
      <div class="confirm-actions">
        <button class="btn-sm" onclick={cancelDelete}>Cancel</button>
        <button class="btn-sm btn-danger" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

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
  .edit-form {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px dashed var(--border, #2d2d44);
  }
  .edit-form h4 {
    margin: 0 0 10px;
    font-size: 0.9rem;
  }
  .edit-form label {
    display: block;
    margin-bottom: 10px;
    font-size: 0.8rem;
    color: var(--muted, #888);
  }
  .edit-form input {
    width: 100%;
    margin-top: 4px;
    padding: 7px 10px;
    border-radius: 6px;
    border: 1px solid var(--border, #2d2d44);
    background: var(--input-bg, #0f0f23);
    color: var(--fg, #e0e0e0);
    font-size: 0.85rem;
    box-sizing: border-box;
  }
  .btn-icon {
    width: 28px;
    text-align: center;
    padding: 4px 0;
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
  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .confirm-dialog {
    background: var(--card-bg, #1a1a2e);
    border: 1px solid var(--border, #2d2d44);
    border-radius: 8px;
    padding: 20px;
    max-width: 360px;
    width: 90%;
  }
  .confirm-dialog h3 {
    margin: 0 0 8px;
    font-size: 1rem;
  }
  .confirm-dialog p {
    color: var(--muted, #888);
    font-size: 0.85rem;
    margin: 0 0 16px;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
