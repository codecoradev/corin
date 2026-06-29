<script lang="ts">
  import { onMount } from 'svelte';
  import { system, updater, agents as agentApi } from '../ts/ipc';
  import type { Update } from '@tauri-apps/plugin-updater';

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  // ─── Active tab ───
  type Tab = 'corin' | 'general' | 'data' | 'agents';
  let activeTab = $state<Tab>('corin');

  // ─── Settings state ───
  let settings = $state<Record<string, string>>({});
  let loading = $state(true);
  let saving = $state(false);
  let savedMsg = $state(false);

  // Editable local copies
  let theme = $state('catppuccin-mocha');
  let defaultNamespace = $state('');
  let maxResults = $state('50');

  // ─── Update state ───
  let checkingUpdates = $state(false);
  let installingUpdate = $state(false);
  let updateStatus = $state<string | null>(null);
  let pendingUpdate: Update | null = $state(null);

  // ─── Data dir info ───
  let dataDir = $state<string | null>(null);

  // ─── AI Agents (#55) ───
  let detectedAgents = $state<Array<{ name: string; config_path: string; found: boolean }>>([]);
  let generatingMd = $state(false);
  let agentMdPath = $state<string | null>(null);
  let runningDream = $state(false);
  let dreamResult = $state<{ success: boolean; result: unknown } | null>(null);

  async function loadAgents() {
    try {
      detectedAgents = await agentApi.detect();
    } catch { /* ignore */ }
  }

  async function genAgentMd() {
    generatingMd = true;
    try { agentMdPath = await agentApi.generateAgentMd(); } catch { /* ignore */ }
    generatingMd = false;
  }

  async function runDream() {
    runningDream = true;
    try { dreamResult = await agentApi.runDream(); } catch { /* ignore */ }
    runningDream = false;
  }

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

  async function loadDataDir() {
    try {
      dataDir = await system.openDataDir();
    } catch {
      dataDir = null;
    }
  }

  onMount(() => {
    loadSettings();
    loadDataDir();
    loadAgents();
  });

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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: 'corin', label: 'CorIn', icon: '◧' },
    { id: 'general', label: 'General', icon: '⚙' },
    { id: 'data', label: 'Data', icon: '▤' },
    { id: 'agents', label: 'AI Agents', icon: '◈' },
  ];
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<div class="backdrop" onclick={onclose} role="presentation"></div>

<!-- Modal -->
<div class="modal" role="dialog" aria-modal="true" aria-label="Settings">
  <header class="modal-header">
    <h2>⚙ Settings</h2>
    <button class="close-btn" onclick={onclose} aria-label="Close settings" title="Close (Esc)">
      ✕
    </button>
  </header>

  <div class="modal-body">
    <!-- Sidebar -->
    <aside class="settings-sidebar">
      {#each tabs as tab}
        <button
          class="tab-btn"
          class:active={activeTab === tab.id}
          onclick={() => (activeTab = tab.id)}
        >
          <span class="tab-icon">{tab.icon}</span>
          <span class="tab-label">{tab.label}</span>
        </button>
      {/each}

      <div class="sidebar-separator"></div>

      <div class="sidebar-info">
        <p class="version">CorIn v0.1.1</p>
        <p class="powered">codecora.dev</p>
      </div>
    </aside>

    <!-- Content -->
    <div class="settings-content">
      {#if loading}
        <div class="loading">Loading...</div>
      {:else if activeTab === 'corin'}
        <!-- CorIn Preferences -->
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
            {#if savedMsg}
              <span class="saved-msg">✓ Saved</span>
            {/if}
          </div>
        </section>

        <section class="content-section">
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
        </section>

      {:else if activeTab === 'general'}
        <!-- General / System-wide -->
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
            <div class="shortcut-row">
              <span>Toggle Sidebar</span>
              <kbd>Ctrl+B</kbd>
            </div>
            <div class="shortcut-row">
              <span>New Memory</span>
              <kbd>Ctrl+N</kbd>
            </div>
            <div class="shortcut-row">
              <span>Close Settings</span>
              <kbd>Esc</kbd>
            </div>
          </div>
        </section>

      {:else if activeTab === 'data'}
        <!-- Data Management -->
        <section class="content-section">
          <h3>Data Directory</h3>
          <div class="data-dir-info">
            <p class="data-dir-label">CorIn stores data at:</p>
            <code class="data-dir-path">{dataDir ?? '~/.codecora/corin/'}</code>
          </div>
        </section>

        <section class="content-section">
          <h3>Export</h3>
          <div class="data-actions">
            <button class="data-btn" onclick={() => handleExport('json')}>
              ↓ Export JSON
            </button>
            <button class="data-btn" onclick={() => handleExport('markdown')}>
              ↓ Export Markdown
            </button>
          </div>
        </section>

      {:else if activeTab === 'agents'}
        <!-- AI Agent Integration (#55) -->
        <section class="content-section">
          <h3>Detected Agents</h3>
          <p class="setting-hint">AI coding agents that can use uteke memory.</p>
          <div class="agent-list">
            {#each detectedAgents as agent}
              <div class="agent-item" class:found={agent.found}>
                <span class="agent-status">{agent.found ? '✓' : '○'}</span>
                <div class="agent-info">
                  <span class="agent-name">{agent.name}</span>
                  <code class="agent-path">{agent.config_path}</code>
                </div>
              </div>
            {/each}
          </div>
        </section>

        <section class="content-section">
          <h3>Agent Instructions (.agent.md)</h3>
          <p class="setting-hint">Generate a <code>.agent.md</code> file with memory protocol instructions for AI agents.</p>
          <button class="data-btn" onclick={genAgentMd} disabled={generatingMd}>
            {generatingMd ? 'Generating...' : '✦ Generate .agent.md'}
          </button>
          {#if agentMdPath}
            <p class="setting-hint" style="margin-top:8px;">
              ✓ Written to <code>{agentMdPath}</code>
            </p>
          {/if}
        </section>

        <section class="content-section">
          <h3>Dream Cycle (Maintenance)</h3>
          <p class="setting-hint">Run uteke's maintenance pipeline: lint → backlinks → dedup → orphans → compact.</p>
          <button class="data-btn" onclick={runDream} disabled={runningDream}>
            {runningDream ? 'Running...' : '☾ Run Dream Cycle'}
          </button>
          {#if dreamResult}
            <div class="dream-result">
              {#if dreamResult.success}
                <span style="color: var(--green);">✓ Dream cycle completed</span>
              {:else}
                <span style="color: var(--red);">✗ Failed or partial</span>
              {/if}
            </div>
          {/if}
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
    z-index: 100;
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 720px;
    max-width: 92vw;
    height: 480px;
    max-height: 85vh;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    z-index: 101;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .modal-header h2 {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1rem;
    padding: 4px 8px;
    border-radius: 4px;
    line-height: 1;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .modal-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  /* ─── Sidebar ─── */
  .settings-sidebar {
    width: 180px;
    flex-shrink: 0;
    background: var(--bg-tertiary);
    border-right: 1px solid var(--border);
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.85rem;
    border-radius: 4px;
    text-align: left;
    width: 100%;
    transition: background 0.1s;
  }
  .tab-btn:hover {
    background: var(--bg-hover);
  }
  .tab-btn.active {
    background: var(--bg-hover);
    color: var(--accent);
    font-weight: 600;
  }
  .tab-icon {
    width: 18px;
    text-align: center;
    flex-shrink: 0;
  }
  .tab-label {
    white-space: nowrap;
  }

  .sidebar-separator {
    height: 1px;
    background: var(--border);
    margin: 8px 4px;
  }

  .sidebar-info {
    margin-top: auto;
    padding: 8px 12px;
  }
  .sidebar-info .version {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0;
  }
  .sidebar-info .powered {
    font-size: 0.7rem;
    color: var(--text-muted);
    margin: 2px 0 0;
    opacity: 0.7;
  }

  /* ─── Content ─── */
  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px;
  }

  .content-section {
    margin-bottom: 24px;
  }
  .content-section:last-child {
    margin-bottom: 0;
  }

  .content-section h3 {
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin: 0 0 14px;
    font-weight: 600;
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
    width: 220px;
    padding: 6px 10px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.85rem;
    outline: none;
  }
  .setting-row select:focus,
  .setting-row input:focus {
    border-color: var(--accent);
  }
  .setting-row select:disabled {
    opacity: 0.5;
  }

  .action-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }

  .save-btn {
    padding: 8px 20px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    font-weight: 600;
    font-size: 0.85rem;
  }
  .save-btn:disabled {
    opacity: 0.5;
  }

  .saved-msg {
    color: var(--green);
    font-size: 0.85rem;
  }

  /* ─── Data tab ─── */
  .data-dir-info {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 12px 14px;
  }
  .data-dir-label {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0 0 6px;
  }
  .data-dir-path {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--accent);
    word-break: break-all;
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
    font-size: 0.85rem;
  }
  .data-btn:hover {
    border-color: var(--accent);
  }

  /* ─── About tab ─── */
  .about-info {
    color: var(--text-muted);
    font-size: 0.85rem;
    line-height: 1.6;
  }
  .about-info p {
    margin: 0 0 4px;
  }
  .about-info strong {
    color: var(--text-primary);
  }
  .about-info .hint {
    margin-top: 8px;
    font-size: 0.8rem;
    opacity: 0.7;
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .shortcut-row kbd {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    padding: 2px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
  }

  /* ─── Updates ─── */
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

  .agent-list { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
  .agent-item { display: flex; align-items: flex-start; gap: 10px; padding: 8px 12px; border-radius: 6px; background: var(--bg-secondary, rgba(255,255,255,0.03)); }
  .agent-item.found { background: rgba(166,227,161,0.08); }
  .agent-status { font-size: 1.1rem; font-weight: 700; min-width: 20px; }
  .agent-item.found .agent-status { color: var(--green); }
  .agent-info { display: flex; flex-direction: column; gap: 2px; }
  .agent-name { font-weight: 600; font-size: 0.9rem; }
  .agent-path { font-size: 0.75rem; color: var(--text-muted); }
  .dream-result { margin-top: 8px; font-size: 0.85rem; }
</style>
