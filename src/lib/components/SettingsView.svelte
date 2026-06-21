<script lang="ts">
  import { onMount } from 'svelte';
  import { system } from '../ts/ipc';

  let settings = $state<Record<string, string>>({});
  let loading = $state(true);
  let saving = $state(false);
  let savedMsg = $state(false);

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
      <h3>About</h3>
      <div class="about-info">
        <p><strong>Codecora Hub</strong> v0.1.0</p>
        <p>Desktop knowledge workstation powered by codecora.dev</p>
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

  .loading {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }
</style>
