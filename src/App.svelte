<script lang="ts">
  import './app.css';
  import { onMount } from 'svelte';
  import { system, uteke } from './lib/ts/ipc';
  import type { View, MemoryEntry } from './lib/ts/types';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Dashboard from './lib/components/Dashboard.svelte';
  import MemoryList from './lib/components/MemoryList.svelte';
  import MemoryDetail from './lib/components/MemoryDetail.svelte';
  import MemoryEditor from './lib/components/MemoryEditor.svelte';
  import GraphView from './lib/components/GraphView.svelte';
  import RoomsView from './lib/components/RoomsView.svelte';
  import SettingsView from './lib/components/SettingsView.svelte';

  // App state
  let dataDirInitialized = $state(false);
  let dataDir = $state<string | null>(null);
  let activeView = $state<View>('dashboard');
  let selectedMemoryId = $state<string | null>(null);
  let sidebarCollapsed = $state(false);
  let namespace = $state<string | null>(null);
  let namespaces = $state<string[]>([]);
  let showEditor = $state(false);
  let editorMemory = $state<MemoryEntry | null>(null);
  let searchQuery = $state<string | null>(null);

  async function initDataDir() {
    try {
      const dir = await system.openDataDir();
      dataDir = dir;
      dataDirInitialized = true;
      await loadNamespaces();
    } catch (e) {
      console.error('Failed to init data dir:', e);
    }
  }

  async function loadNamespaces() {
    try {
      // Merge namespaces from Hub DB + Uteke DB (if available)
      const hubNs = await system.listNamespaces().catch(() => []);
      let utekeNs: string[] = [];
      if (await uteke.available().catch(() => false)) {
        utekeNs = await uteke.namespaces().catch(() => []);
      }
      namespaces = [...new Set([...hubNs, ...utekeNs])].sort();
    } catch {
      namespaces = [];
    }
  }

  function navigate(view: View) {
    activeView = view;
    selectedMemoryId = null;
    searchQuery = null;
  }

  function selectMemory(id: string) {
    selectedMemoryId = id;
  }

  function newMemory() {
    editorMemory = null;
    showEditor = true;
  }

  function editMemory(m: MemoryEntry) {
    editorMemory = m;
    showEditor = true;
  }

  function closeEditor() {
    showEditor = false;
    editorMemory = null;
  }

  function handleSave() {
    showEditor = false;
    editorMemory = null;
    // Trigger re-render by toggling a state
    if (activeView === 'dashboard' || activeView === 'memories') {
      refreshKey++;
    }
  }

  let refreshKey = $state(0);

  function quickSearch(query: string) {
    searchQuery = query;
    activeView = 'memories';
  }

  function handleKeydown(e: KeyboardEvent) {
    // Ctrl+B: toggle sidebar
    if (e.ctrlKey && e.key === 'b') {
      e.preventDefault();
      sidebarCollapsed = !sidebarCollapsed;
    }
    // Ctrl+N: new memory (only if not already in editor)
    if (e.ctrlKey && e.key === 'n' && !showEditor) {
      e.preventDefault();
      newMemory();
    }
  }

  onMount(async () => {
    window.addEventListener('keydown', handleKeydown);
    // Auto-init data directory on startup
    // setup() hook in lib.rs already initializes ~/.codecora/
    // This call just ensures the connection is stored in state
    await initDataDir();
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if !dataDirInitialized}
  <div class="welcome-screen">
    <div class="welcome-content">
      <div class="logo">◆</div>
      <h1>Codecora Hub</h1>
      <p>Desktop knowledge workstation powered by codecora.dev</p>
      <button class="primary-btn" onclick={initDataDir}>Initialize Workspace</button>
      <p class="hint">Data will be stored in <code>~/.codecora/hub/</code></p>
    </div>
  </div>
{:else}
  <div class="app-layout">
    <Sidebar
      activeView={activeView}
      {namespace}
      {namespaces}
      collapsed={sidebarCollapsed}
      onnavigate={navigate}
      onnamespacechange={(ns) => {
        namespace = ns;
      }}
      onnewmemory={newMemory}
      oncollapse={() => (sidebarCollapsed = !sidebarCollapsed)}
    />

    <main class="main-content">
      {#if selectedMemoryId}
        <MemoryDetail
          memoryId={selectedMemoryId}
          onedit={editMemory}
          onback={() => (selectedMemoryId = null)}
          onneighborclick={selectMemory}
        />
      {:else if activeView === 'dashboard'}
        <Dashboard {namespace} onmemoryclick={selectMemory} onquicksearch={quickSearch} />
      {:else if activeView === 'memories'}
        {#key refreshKey}
          <MemoryList {namespace} onmemoryclick={selectMemory} onnewmemory={newMemory} />
        {/key}
      {:else if activeView === 'graph'}
        <GraphView {namespace} onmemoryclick={selectMemory} />
      {:else if activeView === 'rooms'}
        <RoomsView {namespace} oncreateroom={newMemory} />
      {:else if activeView === 'settings'}
        <SettingsView />
      {/if}
    </main>
  </div>
{/if}

{#if showEditor}
  <MemoryEditor
    memory={editorMemory}
    {namespace}
    onsave={handleSave}
    onclose={closeEditor}
  />
{/if}

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-primary);
  }

  .welcome-screen {
    width: 100%;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
  }

  .welcome-content {
    text-align: center;
    max-width: 400px;
  }

  .logo {
    font-size: 3rem;
    color: var(--accent);
    margin-bottom: 12px;
  }

  .welcome-content h1 {
    font-size: 2rem;
    font-weight: 700;
    margin-bottom: 0.5rem;
    color: var(--accent);
  }

  .welcome-content p {
    color: var(--text-muted);
    margin-bottom: 1.5rem;
  }

  .primary-btn {
    padding: 10px 24px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 6px;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .primary-btn:hover {
    opacity: 0.85;
  }

  .hint {
    font-size: 0.8rem;
    margin-top: 0.75rem;
  }

  .hint code {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    color: var(--accent);
  }
</style>
