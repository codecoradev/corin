<script lang="ts">
  import './app.css';
  import { onMount } from 'svelte';
  import { system } from './lib/ts/ipc';
  import type { View, MemoryEntry } from './lib/ts/types';
  import { pendingDocSlug } from './lib/stores/nav';
	import { theme } from './lib/stores/theme.svelte';
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
  import LifecycleView from './lib/components/LifecycleView.svelte';
  import ToolsView from './lib/components/ToolsView.svelte';
  import { Notification } from './lib/ui';
  import { toastStore } from './lib/ui';
  import { fadeQuick, overlayFade, overlayFlyUp } from './lib/transitions';
  import DetailPanel from './lib/components/DetailPanel.svelte';
  import { isWebMode } from './lib/ts/transport';

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
      if (isWebMode) {
        toastStore.error(String(e instanceof Error ? e.message : e));
      }
    }
  }

  function navigate(view: View) {
    // Settings is a modal popup, not a full view — the page behind it
    // must stay exactly as it is.
    if (view === 'settings') {
      showSettings = true;
      return;
    }

    activeView = view;
    searchQuery = null;

    // Deep-linkable views (web mode): keep the hash in sync so a view can
    // be opened directly via #memories, #lifecycle, etc.
    if (isWebMode && location.hash !== `#${view}`) {
      history.replaceState(null, '', `#${view}`);
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
    const wasEditing = !!editorMemory;
    showEditor = false;
    editorMemory = null;
    refreshKey++;
    toastStore.success(wasEditing ? 'Memory updated' : 'Memory created');
  }

  // Memory deleted from the detail panel — refresh the list underneath and
  // confirm to the user (the panel closing alone is ambiguous).
  function handleMemoryDeleted() {
    refreshKey++;
    detailId = null;
    toastStore.success('Memory deleted');
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
      toggleSidebar();
    }
    if (e.ctrlKey && e.key === 'n' && !showEditor) {
      e.preventDefault();
      newMemory();
    }
  }

  // Auto-collapse to the icon rail on narrow viewports (web in a small
  // window, narrow desktop windows). A manual toggle cancels the auto
  // restore so we never fight the user's explicit choice.
  let autoCollapsed = false;

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    autoCollapsed = false;
  }

  function handleViewportChange() {
    if (window.innerWidth < 900 && !sidebarCollapsed) {
      sidebarCollapsed = true;
      autoCollapsed = true;
    } else if (window.innerWidth >= 900 && autoCollapsed) {
      sidebarCollapsed = false;
      autoCollapsed = false;
    }
  }

  const VALID_HASH_VIEWS: View[] = [
    'dashboard', 'memories', 'namespaces', 'graph', 'rooms', 'documents', 'lifecycle', 'tools',
  ];

  function viewFromHash(): View | null {
    const h = location.hash.replace('#', '');
    return (VALID_HASH_VIEWS as string[]).includes(h) ? (h as View) : null;
  }

  function handleHashChange() {
    const v = viewFromHash();
    if (v && v !== activeView) navigate(v);
  }

  onMount(() => {
		theme.init();
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('hashchange', handleHashChange);
    window.addEventListener('resize', handleViewportChange);
    const initial = viewFromHash();
    if (initial) activeView = initial;
    handleViewportChange();
    initDataDir();
    return () => {
      window.removeEventListener('keydown', handleKeydown);
      window.removeEventListener('hashchange', handleHashChange);
      window.removeEventListener('resize', handleViewportChange);
    };
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
      oncollapse={toggleSidebar}
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
              <Dashboard {namespace} onmemoryclick={openDetail} onquicksearch={quickSearch} onnewmemory={newMemory} />
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
          {:else if activeView === 'lifecycle'}
            <LifecycleView {namespace} onmemoryclick={openDetail} />
          {:else if activeView === 'tools'}
            <ToolsView />
          {/if}
        </div>
      {/key}
    </main>
  </div>
{/if}

<!-- Universal slide-in detail panel (used by all views) -->
{#if detailId}
  <div transition:overlayFade>
    <DetailPanel memoryId={detailId} onclose={closeDetail} onneighborclick={detailNavigate} onedit={editMemory}>
      <MemoryDetail
        memoryId={detailId}
        onback={closeDetail}
        onneighborclick={detailNavigate}
        onedit={editMemory}
        ondeleted={handleMemoryDeleted}
      />
    </DetailPanel>
  </div>
{/if}

{#if showSettings}
  <div transition:overlayFade>
    <SettingsModal onclose={closeSettings} />
  </div>
{/if}

{#if showEditor}
  <div transition:overlayFlyUp>
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
    border-radius: var(--radius);
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s var(--ease-out), transform 0.08s var(--ease-out);
  }

  .primary-btn:hover {
    filter: brightness(1.12);
  }
  .primary-btn:active {
    transform: translateY(1px);
  }
</style>
