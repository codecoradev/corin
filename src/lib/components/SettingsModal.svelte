<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { open as shellOpen } from '@tauri-apps/plugin-shell';
  import { system } from '../ts/ipc';
  import ImportExport from './ImportExport.svelte';
  import AgentsSection from './settings/AgentsSection.svelte';
  import UpdatesSection from './settings/UpdatesSection.svelte';

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  type Tab = 'corin' | 'general' | 'data' | 'connections' | 'agents';
  let activeTab = $state<Tab>('corin');

  // Settings state
  let settings = $state<Record<string, string>>({});
  let loading = $state(true);
  let saving = $state(false);
  let savedMsg = $state(false);
  let theme = $state('catppuccin-mocha');
  let defaultNamespace = $state('');
  let maxResults = $state('50');

  // Data dir
  let dataDir = $state<string | null>(null);
  let namespaces = $state<string[]>([]);
  let appVersion = $state('...');

  async function loadSettings() {
    loading = true;
    try {
      settings = await system.getSettings();
      theme = settings['theme'] ?? 'catppuccin-mocha';
      defaultNamespace = settings['default_namespace'] ?? '';
      maxResults = settings['max_results'] ?? '50';
    } catch { /* not initialized */ }
    finally { loading = false; }
  }

  async function loadDataDir() {
    try { dataDir = await system.openDataDir(); } catch { dataDir = null; }
    try { namespaces = await system.listNamespaces(); } catch { namespaces = []; }
  }

  onMount(async () => {
    loadSettings();
    loadDataDir();
    try { appVersion = await getVersion(); } catch { appVersion = 'dev'; }
  });

  async function handleSave() {
    saving = true;
    try {
      await system.setSettings({ theme, default_namespace: defaultNamespace, max_results: maxResults });
      savedMsg = true;
      setTimeout(() => (savedMsg = false), 2000);
    } catch { /* ignore */ }
    finally { saving = false; }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); onclose(); }
  }

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'corin', label: 'CorIn', icon: '◧' },
    { id: 'general', label: 'General', icon: '⚙' },
    { id: 'data', label: 'Data', icon: '▤' },
    { id: 'connections', label: 'Connections', icon: '☍' },
    { id: 'agents', label: 'AI Agents', icon: '◈' },
  ];
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="backdrop" onclick={onclose} role="presentation"></div>

<div class="modal" role="dialog" aria-modal="true" aria-label="Settings">
  <header class="modal-header">
    <h2>⚙ Settings</h2>
    <button class="close-btn" onclick={onclose} aria-label="Close settings" title="Close (Esc)">✕</button>
  </header>

  <div class="modal-body">
    <aside class="settings-sidebar">
      {#each tabs as tab}
        <button class="tab-btn" class:active={activeTab === tab.id} onclick={() => (activeTab = tab.id)}>
          <span class="tab-icon">{tab.icon}</span>
          <span class="tab-label">{tab.label}</span>
        </button>
      {/each}
      <div class="sidebar-separator"></div>
      <div class="sidebar-info">
        <p class="version">CorIn v{appVersion}</p>
        <a
          class="powered"
          href="https://codecora.dev"
          target="_blank"
          rel="noopener"
          onclick={(e) => {
            e.preventDefault();
            shellOpen('https://codecora.dev');
          }}>codecora.dev</a>
      </div>
    </aside>

    <div class="settings-content">
      {#if loading}
        <div class="loading">Loading...</div>
      {:else if activeTab === 'corin'}
        <section class="content-section">
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
          <div class="action-row">
            <button class="save-btn" onclick={handleSave} disabled={saving}>
              {saving ? 'Saving...' : 'Save Settings'}
            </button>
            {#if savedMsg}<span class="saved-msg">✓ Saved</span>{/if}
          </div>
        </section>

        <UpdatesSection />

      {:else if activeTab === 'general'}
        <section class="content-section">
          <h3>About</h3>
          <div class="about-info">
            <p><strong>CorIn</strong> — Cora Intelligence</p>
            <p>Desktop knowledge workstation powered by codecora.dev</p>
            <p class="hint">Local-first, offline-capable. Your data never leaves this machine.</p>
          </div>
        </section>

        <section class="content-section">
          <h3>Keyboard Shortcuts</h3>
          <div class="shortcuts">
            <div class="shortcut-row"><span>Toggle Sidebar</span><kbd>Ctrl+B</kbd></div>
            <div class="shortcut-row"><span>New Memory</span><kbd>Ctrl+N</kbd></div>
            <div class="shortcut-row"><span>Close Settings</span><kbd>Esc</kbd></div>
          </div>
        </section>

      {:else if activeTab === 'data'}
        <section class="content-section">
          <h3>Data Directory</h3>
          <div class="data-dir-info">
            <p class="data-dir-label">CorIn stores data at:</p>
            <code class="data-dir-path">{dataDir ?? '~/.codecora/corin/'}</code>
          </div>
        </section>

        <section class="content-section">
          <h3>Import / Export</h3>
          <ImportExport {namespaces} />
        </section>

      {:else if activeTab === 'connections'}
        {#await import('./ConnectionManager.svelte') then module}
          <module.default />
        {/await}

      {:else if activeTab === 'agents'}
        <AgentsSection />
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); backdrop-filter: blur(2px); z-index: 100; }

  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    width: 720px; max-width: 92vw; height: 480px; max-height: 85vh;
    background: var(--bg-secondary); border: 1px solid var(--border);
    border-radius: 8px; display: flex; flex-direction: column;
    overflow: hidden; z-index: 101; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-header { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--border); flex-shrink: 0; }
  .modal-header h2 { font-size: 1.1rem; font-weight: 600; margin: 0; }
  .close-btn { background: transparent; border: none; color: var(--text-muted); font-size: 1rem; padding: 4px 8px; border-radius: 4px; line-height: 1; }
  .close-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .modal-body { display: flex; flex: 1; overflow: hidden; }

  .settings-sidebar { width: 180px; flex-shrink: 0; background: var(--bg-tertiary); border-right: 1px solid var(--border); padding: 12px 8px; display: flex; flex-direction: column; gap: 2px; overflow-y: auto; }
  .tab-btn { display: flex; align-items: center; gap: 10px; padding: 8px 12px; background: transparent; border: none; color: var(--text-secondary); font-size: 0.85rem; border-radius: 4px; text-align: left; width: 100%; transition: background 0.1s; }
  .tab-btn:hover { background: var(--bg-hover); }
  .tab-btn.active { background: var(--bg-hover); color: var(--accent); font-weight: 600; }
  .tab-icon { width: 18px; text-align: center; flex-shrink: 0; }
  .tab-label { white-space: nowrap; }
  .sidebar-separator { height: 1px; background: var(--border); margin: 8px 4px; }
  .sidebar-info { margin-top: auto; padding: 8px 12px; }
  .sidebar-info .version { font-size: 0.75rem; color: var(--text-muted); margin: 0; }
  .sidebar-info .powered { font-size: 0.7rem; color: var(--text-muted); margin: 2px 0 0; opacity: 0.7; text-decoration: none; cursor: pointer; display: inline-block; }
  .sidebar-info .powered:hover { opacity: 1; color: var(--accent); }

  .settings-content { flex: 1; overflow-y: auto; padding: 20px 24px; }
  .content-section { margin-bottom: 24px; }
  .content-section:last-child { margin-bottom: 0; }
  .content-section h3 { font-size: 0.9rem; color: var(--text-secondary); margin: 0 0 14px; font-weight: 600; }

  .setting-row { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .setting-row label { font-size: 0.85rem; color: var(--text-secondary); }
  .setting-row select, .setting-row input { width: 220px; padding: 6px 10px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px; font-size: 0.85rem; outline: none; }
  .setting-row select:focus, .setting-row input:focus { border-color: var(--accent); }
  .setting-row select:disabled { opacity: 0.5; }

  .action-row { display: flex; align-items: center; gap: 12px; margin-top: 12px; }
  .save-btn { padding: 8px 20px; background: var(--accent); color: var(--bg-primary); border: none; border-radius: 4px; font-weight: 600; font-size: 0.85rem; }
  .save-btn:disabled { opacity: 0.5; }
  .saved-msg { color: var(--green); font-size: 0.85rem; }

  .data-dir-info { background: var(--bg-primary); border: 1px solid var(--border); border-radius: 4px; padding: 12px 14px; }
  .data-dir-label { font-size: 0.8rem; color: var(--text-muted); margin: 0 0 6px; }
  .data-dir-path { font-family: var(--font-mono); font-size: 0.8rem; color: var(--accent); word-break: break-all; }

  .about-info { color: var(--text-muted); font-size: 0.85rem; line-height: 1.6; }
  .about-info p { margin: 0 0 4px; }
  .about-info strong { color: var(--text-primary); }
  .about-info .hint { margin-top: 8px; font-size: 0.8rem; opacity: 0.7; }

  .shortcuts { display: flex; flex-direction: column; gap: 8px; }
  .shortcut-row { display: flex; align-items: center; justify-content: space-between; font-size: 0.85rem; color: var(--text-secondary); }
  .shortcut-row kbd { font-family: var(--font-mono); font-size: 0.75rem; padding: 2px 8px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); }

  .loading { text-align: center; padding: 40px; color: var(--text-muted); }
</style>
