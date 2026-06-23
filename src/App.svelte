<script lang="ts">
  import './app.css';
  import { onMount } from 'svelte';
  import { system } from './lib/ts/ipc';
  import type { View, MemoryEntry } from './lib/ts/types';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Dashboard from './lib/components/Dashboard.svelte';
  import MemoryList from './lib/components/MemoryList.svelte';
  import MemoryDetail from './lib/components/MemoryDetail.svelte';
  import MemoryEditor from './lib/components/MemoryEditor.svelte';
  import GraphView from './lib/components/GraphView.svelte';
  import RoomsView from './lib/components/RoomsView.svelte';
  import SettingsModal from './lib/components/SettingsModal.svelte';
  import NamespacesView from './lib/components/NamespacesView.svelte';

  // App state
  let dataDirInitialized = $state(false);
  let dataDir = $state<string | null>(null);
  let activeView = $state<View>('dashboard');
  let selectedMemoryId = $state<string | null>(null);
  let sidebarCollapsed = $state(false);
  let namespace = $state<string | null>(null);
  let showEditor = $state(false);
  let showSettings = $state(false);
  let graphDetailId = $state<string | null>(null);
  let editorMemory = $state<MemoryEntry | null>(null);
  let searchQuery = $state<string | null>(null);

  async function initDataDir() {
    try {
      const dir = await system.openDataDir();
      dataDir = dir;
      dataDirInitialized = true;
    } catch (e) {
      console.error('Failed to init data dir:', e);
    }
  }

  function navigate(view: View) {
    activeView = view;
    selectedMemoryId = null;
    searchQuery = null;

    // Settings is now a modal popup, not a full view.
    if (view === 'settings') {
      showSettings = true;
      return;
    }
  }

  function selectMemory(id: string) {
    selectedMemoryId = id;
  }

  // From graph: open detail as overlay (don't unmount the graph)
  function openGraphDetail(id: string) {
    graphDetailId = id;
  }

  function closeGraphDetail() {
    graphDetailId = null;
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

  function closeSettings() {
    showSettings = false;
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

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
    // Auto-init data directory on startup
    // setup() hook in lib.rs already initializes ~/.codecora/
    // This call just ensures the connection is stored in state
    initDataDir();
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if !dataDirInitialized}
  <div class="welcome-screen">
    <div class="welcome-content">
      <img src="/corin-logo.png" alt="CorIn" class="welcome-logo" />
      <h1>CorIn</h1>
      <p>Cora Intelligence — desktop knowledge workstation</p>
      <button class="primary-btn" onclick={initDataDir}>Initialize Workspace</button>
      <p>Data will be stored in <code>~/.codecora/corin/</code></p>
    </div>
  </div>
{:else}
  <div class="app-layout">
    <Sidebar
      activeView={activeView}
      collapsed={sidebarCollapsed}
      onnavigate={navigate}
      onnewmemory={newMemory}
      oncollapse={() => (sidebarCollapsed = !sidebarCollapsed)}
    />

    <main class="main-content">
      {#if selectedMemoryId}
        <MemoryDetail
          memoryId={selectedMemoryId}
          onedit={editMemory}
          onback={() => {
            selectedMemoryId = null;
            refreshKey++;
          }}
          onneighborclick={selectMemory}
        />
      {:else if activeView === 'dashboard'}
        {#key refreshKey}
          <Dashboard {namespace} onmemoryclick={selectMemory} onquicksearch={quickSearch} />
        {/key}
      {:else if activeView === 'memories'}
        {#key refreshKey}
          <MemoryList {namespace} onmemoryclick={selectMemory} onnewmemory={newMemory} />
        {/key}
      {:else if activeView === 'namespaces'}
        <NamespacesView
          onmemoryclick={selectMemory}
        />
      {:else if activeView === 'graph'}
        <GraphView onmemoryclick={openGraphDetail} />
      {:else if activeView === 'rooms'}
        <RoomsView {namespace} onmemoryclick={selectMemory} />
      {/if}
    </main>
  </div>
{/if}

{#if showSettings}
  <SettingsModal onclose={closeSettings} />
{/if}

{#if graphDetailId}
  <div class="graph-detail-overlay">
    <MemoryDetail
      memoryId={graphDetailId}
      onedit={editMemory}
      onback={closeGraphDetail}
      onneighborclick={(id) => { graphDetailId = id; }}
    />
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

  .welcome-logo {
    width: 64px;
    height: 64px;
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

  .graph-detail-overlay {
    position: fixed;
    top: 0;
    right: 0;
    width: 480px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.3);
    z-index: 90;
    overflow-y: auto;
  }
</style>
