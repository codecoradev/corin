<script lang="ts">
  import './app.css';
  import { onMount } from 'svelte';
  import { system } from './lib/ts/ipc';
  import type { View, MemoryEntry } from './lib/ts/types';
  import { pendingDocSlug } from './lib/stores/nav';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Dashboard from './lib/components/Dashboard.svelte';
  import MemoryList from './lib/components/MemoryList.svelte';
  import MemoryDetail from './lib/components/MemoryDetail.svelte';
  import MemoryEditor from './lib/components/MemoryEditor.svelte';
  import GraphView from './lib/components/GraphView.svelte';
  import RoomsView from './lib/components/RoomsView.svelte';
  import SettingsModal from './lib/components/SettingsModal.svelte';
  import NamespacesView from './lib/components/NamespacesView.svelte';
  import DocumentsView from './lib/components/DocumentsView.svelte';
  import { Notification } from './lib/ui';
  import { toastStore } from './lib/ui';
  import { fadeQuick } from './lib/transitions';
  import { fade, fly } from 'svelte/transition';
  import DetailPanel from './lib/components/DetailPanel.svelte';

  // App state
  let dataDirInitialized = $state(false);
  let dataDir = $state<string | null>(null);
  let activeView = $state<View>('dashboard');
  let sidebarCollapsed = $state(false);
  let namespace = $state<string | null>(null);

  // Overlay state (views stay mounted underneath)
  let showEditor = $state(false);
  let showSettings = $state(false);
  let detailId = $state<string | null>(null);
  let editorMemory = $state<MemoryEntry | null>(null);
  let searchQuery = $state<string | null>(null);
  let refreshKey = $state(0);

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
    searchQuery = null;

    // Settings is a modal popup, not a full view.
    if (view === 'settings') {
      showSettings = true;
      return;
    }
  }

  // ─── Memory detail (universal slide-in panel) ────────────────────
  // Works from any view: dashboard, memories, namespaces, graph, rooms.
  // The underlying view stays mounted — no re-render when returning.
  function openDetail(id: string) {
    detailId = id;
  }

  // Open a document by slug from elsewhere (e.g. a unified-search doc hit) —
  // stash the slug for DocumentsView to consume on mount, then switch view.
  function openDocument(slug: string) {
    pendingDocSlug.set(slug);
    navigate('documents');
  }

  function closeDetail() {
    detailId = null;
  }

  // Navigate within the detail panel (e.g. click a neighbor)
  function detailNavigate(id: string) {
    detailId = id;
  }

  // When a memory is edited from the detail panel
  function editMemory(m: MemoryEntry) {
    editorMemory = m;
    showEditor = true;
  }

  // ─── Memory editor ────────────────────────────────────────────────
  function newMemory() {
    editorMemory = null;
    showEditor = true;
  }

  function closeEditor() {
    showEditor = false;
    editorMemory = null;
  }

  function handleSave() {
    showEditor = false;
    editorMemory = null;
    refreshKey++;
  }

  function closeSettings() {
    showSettings = false;
  }

  function quickSearch(query: string) {
    searchQuery = query;
    activeView = 'memories';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'b') {
      e.preventDefault();
      sidebarCollapsed = !sidebarCollapsed;
    }
    if (e.ctrlKey && e.key === 'n' && !showEditor) {
      e.preventDefault();
      newMemory();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
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

    <!--
      Main content area — views stay mounted here.
      No view is unmounted when a detail panel opens.
      refreshKey forces re-fetch after editor save.
    -->
    <main class="main-content">
      {#key activeView}
        <div class="view-container" transition:fadeQuick>
          {#if activeView === 'dashboard'}
            {#key refreshKey}
              <Dashboard {namespace} onmemoryclick={openDetail} onquicksearch={quickSearch} />
            {/key}
          {:else if activeView === 'memories'}
            {#key refreshKey}
              <MemoryList {namespace} onmemoryclick={openDetail} onnewmemory={newMemory} ondocumentclick={openDocument} />
            {/key}
          {:else if activeView === 'namespaces'}
            <NamespacesView onmemoryclick={openDetail} />
          {:else if activeView === 'graph'}
            <GraphView onmemoryclick={openDetail} />
          {:else if activeView === 'rooms'}
            <RoomsView {namespace} onmemoryclick={openDetail} />
          {:else if activeView === 'documents'}
            <DocumentsView />
          {/if}
        </div>
      {/key}
    </main>
  </div>
{/if}

<!-- Universal slide-in detail panel (used by all views) -->
{#if detailId}
  <div transition:fade={{ duration: 150 }}>
    <DetailPanel memoryId={detailId} onclose={closeDetail} onneighborclick={detailNavigate} onedit={editMemory}>
      <MemoryDetail
        memoryId={detailId}
        onback={closeDetail}
        onneighborclick={detailNavigate}
        onedit={editMemory}
      />
    </DetailPanel>
  </div>
{/if}

{#if showSettings}
  <div transition:fade={{ duration: 150 }}>
    <SettingsModal onclose={closeSettings} />
  </div>
{/if}

{#if showEditor}
  <div transition:fly={{ duration: 200, y: 20, opacity: 0 }}>
    <MemoryEditor
      memory={editorMemory}
      {namespace}
      onsave={handleSave}
      onclose={closeEditor}
    />
  </div>
{/if}

<!-- Global toast notifications -->
<Notification toasts={toastStore.list} ondismiss={toastStore.dismiss} />

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
    background: var(--bg-primary);
    position: relative;
  }

  .view-container {
    position: absolute;
    inset: 0;
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
</style>
