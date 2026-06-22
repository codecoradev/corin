<script lang="ts">
  import { onMount } from 'svelte';
  import { system, updater, utekeServer } from '../ts/ipc';
  import type { Update } from '@tauri-apps/plugin-updater';

  let settings = $state<Record<string, string>>({});
  let loading = $state(true);
  let saving = $state(false);
  let savedMsg = $state(false);
  let checkingUpdates = $state(false);
  let installingUpdate = $state(false);
  let updateStatus = $state<string | null>(null);
  let pendingUpdate: Update | null = $state(null);

  // Uteke server status
  let serverStatus = $state<{ available: boolean; url?: string; hint?: string; stats?: { total_memories: number; unique_tags: number; hot: number } } | null>(null);

  // Editable local copies
  let theme = $state('catppuccin-mocha');
  let defaultNamespace = $state('');
  let maxResults = $state('50');

  async function loadSettings() {
    loading = true;
    try {
      settings = await system.getSettings();
      theme = settings['theme'] ?? 'catppuccin-mocha';
      defaultNamespace = settings['default_namespace'] ?? '';
      maxResults = settings['max_results'] ?? '50';
    } catch {
      // not initialized
    } finally {
      loading = false;
    }
  }

  onMount(loadSettings);

  // Check uteke-serve status
  utekeServer.status().then((s) => (serverStatus = s)).catch(() => {});

  async function handleSave() {
    saving = true;
    try {
      await system.setSettings({
        theme,
        default_namespace: defaultNamespace,
        max_results: maxResults,
      });
      savedMsg = true;
      setTimeout(() => (savedMsg = false), 2000);
    } catch {
      // ignore
    } finally {
      saving = false;
    }
  }

  async function handleExport(format: 'json' | 'markdown') {
    try {
      const data = await system.exportData(format);
      const blob = new Blob([data], { type: format === 'json' ? 'application/json' : 'text/markdown' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `codecora-export.${format === 'json' ? 'json' : 'md'}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      // ignore
    }
  }

  async function checkForUpdates() {
    checkingUpdates = true;
    updateStatus = null;
    pendingUpdate = null;
    try {
      const update = await updater.check();
      if (update) {
        pendingUpdate = update;
        updateStatus = `Update available: v${update.version}`;
      } else {
        updateStatus = 'Up to date ✅';
      }
    } catch (e: unknown) {
      updateStatus = `Error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      checkingUpdates = false;
    }
  }

  async function installUpdate() {
    if (!pendingUpdate) return;
    installingUpdate = true;
    updateStatus = 'Downloading and installing...';
    try {
      await pendingUpdate.downloadAndInstall();
      updateStatus = 'Update installed. Restarting...';
      await new Promise((r) => setTimeout(r, 1500));
      await import('@tauri-apps/plugin-process').then((m) => m.relaunch());
    } catch (e: unknown) {
      updateStatus = `Error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      installingUpdate = false;
    }
  }
</script>

<div class="settings-view">
  <h2>Settings</h2>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else}
    <div class="settings-section">
      <h3>Preferences</h3>

      <div class="setting-row">
        <label for="theme">Theme</label>
        <select id="theme" bind:value={theme} disabled>
          <option value="catppuccin-mocha">Catppuccin Mocha (Dark)</option>
        </select>
      </div>

      <div class="setting-row">
        <label for="default-ns">Default Namespace</label>
        <input id="default-ns" type="text" bind:value={defaultNamespace} placeholder="default" />
      </div>

      <div class="setting-row">
        <label for="max-results">Max Results per Page</label>
        <input id="max-results" type="number" min="10" max="200" bind:value={maxResults} />
      </div>

      <button class="save-btn" onclick={handleSave} disabled={saving}>
        {saving ? 'Saving...' : 'Save Settings'}
      </button>
      {#if savedMsg}
        <span class="saved-msg">✓ Saved</span>
      {/if}
    </div>

    <div class="settings-section">
      <h3>Data Management</h3>
      <div class="data-actions">
        <button class="data-btn" onclick={() => handleExport('json')}>
          ↓ Export JSON
        </button>
        <button class="data-btn" onclick={() => handleExport('markdown')}>
          ↓ Export Markdown
        </button>
      </div>
    </div>

    <div class="settings-section">
      <h3>Uteke Server</h3>
      <div class="server-status">
        {#if serverStatus?.available}
          <div class="status-row">
            <span class="status-dot online">●</span>
            <span>Connected to <strong>{serverStatus.url}</strong></span>
          </div>
          {#if serverStatus.stats}
            <div class="server-stats">
              <span>{serverStatus.stats.total_memories} memories</span>
              <span>·</span>
              <span>{serverStatus.stats.unique_tags} tags</span>
              <span>·</span>
              <span>{serverStatus.stats.hot} hot</span>
            </div>
          {/if}
          <p class="status-hint">Semantic search and auto-linking are active.</p>
        {:else}
          <div class="status-row">
            <span class="status-dot offline">●</span>
            <span>Not running</span>
          </div>
          <p class="status-hint">{serverStatus?.hint ?? 'Run uteke-serve to enable semantic search.'}</p>
        {/if}
      </div>
    </div>

    <div class="settings-section">
      <h3>About</h3>
      <div class="about-info">
        <p><strong>CorIn</strong> v0.1.0</p>
        <p>Desktop knowledge workstation powered by codecora.dev</p>
      </div>
    </div>

    <div class="settings-section">
      <h3>Updates</h3>
      <div class="update-section">
        <button class="data-btn" onclick={checkForUpdates} disabled={checkingUpdates}>
          {checkingUpdates ? 'Checking...' : '↻ Check for Updates'}
        </button>
        {#if updateStatus}
          <p class="update-msg" class:ok={updateStatus.startsWith('Up to date')} class:err={updateStatus.startsWith('Error')}>{updateStatus}</p>
        {/if}
        {#if pendingUpdate}
          <button class="install-btn" onclick={installUpdate} disabled={installingUpdate}>
            {installingUpdate ? 'Installing...' : '↓ Install Update'}
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .settings-view {
    padding: 16px 24px;
    max-width: 600px;
    margin: 0 auto;
  }

  h2 {
    font-size: 1.3rem;
    margin-bottom: 24px;
  }

  .settings-section {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 16px;
  }

  .settings-section h3 {
    font-size: 0.95rem;
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .setting-row label {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .setting-row select,
  .setting-row input {
    width: 200px;
    padding: 6px 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.85rem;
    outline: none;
  }

  .setting-row select:disabled {
    opacity: 0.5;
  }

  .save-btn {
    margin-top: 8px;
    padding: 8px 20px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
  }

  .save-btn:disabled {
    opacity: 0.5;
  }

  .saved-msg {
    margin-left: 12px;
    color: var(--green);
    font-size: 0.85rem;
  }

  .data-actions {
    display: flex;
    gap: 8px;
  }

  .data-btn {
    padding: 8px 16px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .data-btn:hover {
    border-color: var(--accent);
  }

  .about-info {
    color: var(--text-muted);
    font-size: 0.85rem;
    line-height: 1.6;
  }

  .server-status {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--text-primary);
  }

  .status-dot {
    font-size: 0.65rem;
  }

  .status-dot.online {
    color: var(--green);
  }

  .status-dot.offline {
    color: var(--text-muted);
  }

  .server-stats {
    display: flex;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .status-hint {
    font-size: 0.8rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .update-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }

  .update-msg {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .update-msg.ok {
    color: var(--green);
  }

  .update-msg.err {
    color: var(--red);
  }

  .install-btn {
    padding: 8px 16px;
    background: var(--green);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .install-btn:disabled {
    opacity: 0.5;
  }

  .loading {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }
</style>
