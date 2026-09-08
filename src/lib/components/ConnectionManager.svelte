<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { connection, type HealthInfo } from '../ts/ipc';
  import { getConnectionsStore } from '../stores/connections.svelte';
  import type { Component } from 'svelte';
  import {
    Database,
    Server,
    Star,
    Lock,
    Pencil,
    Check,
    X,
    Plus,
  } from 'lucide-svelte';
  import { ConfirmDialog, Spinner, toastStore, Button } from '../ui';

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
      if (result.success) {
        toastStore.success(`Healthy — ${result.latency_ms}ms`);
      } else {
        toastStore.error(`Connection test failed${result.error ? `: ${result.error}` : ''}`);
      }
    } catch (e) {
      healthResults = { ...healthResults, [id]: { success: false, latency_ms: 0, version: null, error: String(e) } };
      toastStore.error(`Connection test failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      testing = null;
    }
  }

  async function reconnectConn(id: string) {
    reconnecting = id;
    try {
      const result = await store.reconnect(id);
      healthResults = { ...healthResults, [id]: result };
      toastStore.success('Reconnected');
    } catch (e) {
      toastStore.error(`Reconnect failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      reconnecting = null;
    }
  }

  async function setPrimary(id: string) {
    try {
      await store.setPrimary(id);
      toastStore.success('Primary connection set');
    } catch (e) {
      toastStore.error(`Failed to set primary: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function disconnectConn() {
    disconnecting = true;
    try {
      await store.disconnect();
      toastStore.info('Memory backend disconnected');
    } catch (e) {
      toastStore.error(`Disconnect failed: ${e instanceof Error ? e.message : String(e)}`);
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
    const { id, name } = pendingDelete;
    pendingDelete = null;
    try {
      await store.remove(id);
      toastStore.success(`Connection \u201c${name}\u201d deleted`);
    } catch (e) {
      toastStore.error(`Failed to delete connection: ${e instanceof Error ? e.message : String(e)}`);
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
      toastStore.success('Connection added');
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
      toastStore.success('Connection updated');
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
      case 'connected': return 'var(--green)';
      case 'disconnected': return 'var(--text-muted)';
      case 'error': return 'var(--red)';
      default: return 'var(--peach)';
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

  // Product icon as a Lucide component (replaces emoji 🔮/📦).
  function productIcon(type: string): typeof Database {
    switch (type) {
      case 'uteke': return Database;
      default: return Server;
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
    <Button variant="primary" onclick={() => showAdd = !showAdd}>
      {#if showAdd}
        <X size={14} strokeWidth={2.5} /> Cancel
      {:else}
        <Plus size={14} strokeWidth={2.5} /> Add Connection
      {/if}
    </Button>
  </div>

  {#if showAdd}
    <div class="add-form card">
      <h4>New Connection</h4>
      {#if addError}
        <div class="error-banner">{addError}</div>
      {/if}
      <label>
        Name
        <input type="text" bind:value={addName} placeholder="My Uteke VPS" autofocus />
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
        <Button variant="primary" onclick={addConn}>Add Connection</Button>
      </div>
    </div>
  {/if}

  {#if loading}
    <p class="loading"><Spinner size={16} /> Loading connections…</p>
  {:else if connections.length === 0}
    <p class="empty">No connections configured.</p>
  {:else}
    <div class="connection-list">
      {#each connections as conn (conn.id)}
        {@const Icon = productIcon(conn.product_type)}
        <div class="connection-card card" class:primary={conn.is_primary}>
          <div class="card-header">
            <span class="icon">
              <Icon size={18} strokeWidth={1.75} />
            </span>
            <div class="info">
              <h4>{conn.name}</h4>
              <span class="url">{conn.url}</span>
            </div>
            <span class="status-badge" style="color: {statusColor(conn.status)}">
              <span class="status-dot" style="background: {statusColor(conn.status)}"></span>
              {statusLabel(conn.status)}
            </span>
          </div>

          <div class="badges">
            {#if conn.is_primary}
              <span class="badge-tag badge-primary">
                <Star size={11} strokeWidth={2.5} /> Primary
              </span>
            {/if}
            {#if conn.has_token}
              <span class="badge-tag badge-token">
                <Lock size={11} strokeWidth={2.5} /> Auth
              </span>
            {/if}
          </div>

          {#if healthResults[conn.id]}
            {@const h = healthResults[conn.id]}
            <div class="health-info" class:success={h.success} class:fail={!h.success}>
              {#if h.success}
                <span><Check size={13} strokeWidth={2.5} /> Healthy{#if h.version} · uteke v{h.version}{/if} — {h.latency_ms}ms</span>
              {:else}
                <span><X size={13} strokeWidth={2.5} /> {h.error || 'Connection failed'}</span>
              {/if}
            </div>
          {/if}

          <div class="card-actions">
            <Button
              size="sm"
              variant="secondary"
              onclick={() => startEdit(conn.id, conn.name, conn.url)}
              disabled={!!editId}
              title="Edit connection"
            ><Pencil size={13} strokeWidth={2} /></Button>
            <Button
              size="sm"
              variant="secondary"
              onclick={() => testConn(conn.id)}
              disabled={testing === conn.id}
            >
              {testing === conn.id ? 'Testing…' : 'Test'}
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onclick={() => reconnectConn(conn.id)}
              disabled={reconnecting === conn.id}
              title="Rebuild the live backend from this connection (no restart)"
            >
              {reconnecting === conn.id ? 'Reconnecting…' : 'Reconnect'}
            </Button>
            {#if conn.is_primary && conn.status === 'connected'}
              <Button
                size="sm"
                variant="secondary"
                onclick={disconnectConn}
                disabled={disconnecting}
                title="Disconnect the active memory backend (recall/search will fail until reconnect)"
              >
                {disconnecting ? 'Disconnecting…' : 'Disconnect'}
              </Button>
            {/if}
            {#if !conn.is_primary && conn.status === 'connected'}
              <Button size="sm" variant="secondary" onclick={() => setPrimary(conn.id)}>
                Set Primary
              </Button>
            {/if}
            <Button size="sm" variant="danger" onclick={() => requestDelete(conn.id, conn.name)}>
              Delete
            </Button>
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
                Auth Token <span class="optional">(leave blank to keep current{conn.has_token ? '' : ''})</span>
                <input type="password" bind:value={editToken} placeholder={conn.has_token ? '•••••••• (set new to replace)' : 'Bearer token (optional)'} />
              </label>
              <div class="form-actions">
                <Button size="sm" variant="secondary" onclick={cancelEdit} disabled={saving}>Cancel</Button>
                <Button variant="primary" onclick={saveEdit} disabled={saving}>
                  {saving ? 'Saving…' : 'Save'}
                </Button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if pendingDelete}
  <ConfirmDialog
    open={!!pendingDelete}
    title="Delete connection?"
    message="\u201c{pendingDelete.name}\u201d — the auth token will be wiped and the connection row removed. This cannot be undone."
    confirmLabel="Delete"
    danger={true}
    onconfirm={confirmDelete}
    oncancel={cancelDelete}
  />
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
  .card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px;
    margin-bottom: 12px;
  }
  .add-form label {
    display: block;
    margin-bottom: 12px;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .add-form input {
    width: 100%;
    margin-top: 4px;
    padding: 8px 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.9rem;
    box-sizing: border-box;
  }
  .add-form input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .optional {
    font-size: 0.75rem;
    opacity: 0.6;
  }
  .error-banner {
    background: var(--color-red-bg);
    border: 1px solid var(--color-red-line);
    border-radius: var(--radius);
    padding: 8px 12px;
    margin-bottom: 12px;
    font-size: 0.85rem;
    color: var(--red);
  }
  .form-actions {
    margin-top: 8px;
    text-align: right;
  }
  .edit-form {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px dashed var(--border);
  }
  .edit-form h4 {
    margin: 0 0 10px;
    font-size: 0.9rem;
  }
  .edit-form label {
    display: block;
    margin-bottom: 10px;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .edit-form input {
    width: 100%;
    margin-top: 4px;
    padding: 7px 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.85rem;
    box-sizing: border-box;
  }
  .edit-form input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .connection-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .connection-card.primary {
    border-color: var(--accent);
  }
  .card-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
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
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .badges {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-top: 4px;
  }
  .badge-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--bg-hover);
    color: var(--text-secondary);
  }
  .badge-primary { color: var(--yellow); }
  .badge-token { color: var(--text-secondary); }
  .health-info {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 6px 10px;
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .health-info span { display: inline-flex; align-items: center; gap: 5px; }
  .health-info.success {
    background: var(--color-green-bg);
    border: 1px solid var(--color-green-line);
    color: var(--green);
  }
  .health-info.fail {
    background: var(--color-red-bg);
    border: 1px solid var(--color-red-line);
    color: var(--red);
  }
  .card-actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .loading, .empty {
    color: var(--text-muted);
    font-size: 0.9rem;
    text-align: center;
    padding: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
</style>
