<script lang="ts">
  import type { View } from '../ts/types';
  import { utekeServer } from '../ts/ipc';
  import {
    LayoutDashboard,
    Brain,
    Boxes,
    Share2,
    MessagesSquare,
    FileText,
    Settings,
    PanelLeftClose,
    PanelLeftOpen,
    Plus,
  } from 'lucide-svelte';

  interface Props {
    activeView: View;
    collapsed: boolean;
    onnavigate: (view: View) => void;
    onnewmemory: () => void;
    oncollapse: () => void;
  }

  let { activeView, collapsed, onnavigate, onnewmemory, oncollapse }: Props = $props();

  // Uteke server status — always visible in sidebar
  let serverOnline = $state(false);
  let serverChecking = $state(true);

  async function checkServer() {
    try {
      const status = await utekeServer.status();
      serverOnline = status.available;
    } catch {
      serverOnline = false;
    } finally {
      serverChecking = false;
    }
  }

  // Check on mount + periodically
  $effect(() => {
    checkServer();
    const interval = setInterval(checkServer, 30_000);
    return () => clearInterval(interval);
  });

  const navItems: { view: View; label: string; icon: IconComp }[] = [
    { view: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { view: 'memories', label: 'Memories', icon: Brain },
    { view: 'namespaces', label: 'Namespaces', icon: Boxes },
    { view: 'graph', label: 'Graph', icon: Share2 },
    { view: 'rooms', label: 'Rooms', icon: MessagesSquare },
    { view: 'documents', label: 'Documents', icon: FileText },
  ];

  const bottomItems: { view: View; label: string; icon: IconComp }[] = [
    { view: 'settings', label: 'Settings', icon: Settings },
  ];

  type IconComp = typeof LayoutDashboard;
  const iconSize = 18;
</script>

<aside class="sidebar" class:collapsed>
  {#if !collapsed}
    <div class="sidebar-header">
      <div class="logo">
        <img src="/corin-logo.png" alt="CorIn" class="logo-img" />
        <span class="logo-text">CorIn</span>
      </div>
    </div>

    <button class="new-memory-btn" onclick={onnewmemory}>
      <Plus size={16} strokeWidth={2.5} />
      <span>New Memory</span>
      <kbd>Ctrl+N</kbd>
    </button>
  {/if}

  <nav class="nav">
    {#each navItems as item (item.view)}
      <button
        class="nav-item"
        class:active={activeView === item.view}
        onclick={() => onnavigate(item.view)}
        title={collapsed ? item.label : ''}
      >
        <span class="nav-icon">
          <item.icon size={iconSize} strokeWidth={1.75} />
        </span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="nav-bottom">
    {#if !collapsed}
      <div class="server-status" class:online={serverOnline} class:offline={!serverOnline}>
        <span class="status-dot"></span>
        {#if serverChecking}
          <span>Connecting...</span>
        {:else if serverOnline}
          <span>Semantic Search</span>
        {:else}
          <span>uteke-serve offline</span>
        {/if}
      </div>
    {:else}
      <div class="server-status-collapsed" class:online={serverOnline} title={serverOnline ? 'Semantic search active' : 'uteke-serve offline'}>
        <span class="status-dot"></span>
      </div>
    {/if}

    {#each bottomItems as item (item.view)}
      <button
        class="nav-item"
        class:active={activeView === item.view}
        onclick={() => onnavigate(item.view)}
        title={collapsed ? item.label : ''}
      >
        <span class="nav-icon">
          <item.icon size={iconSize} strokeWidth={1.75} />
        </span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="sidebar-footer">
    <button class="collapse-btn" onclick={oncollapse} title={collapsed ? 'Expand (Ctrl+B)' : 'Collapse (Ctrl+B)'}>
      {#if collapsed}
        <PanelLeftOpen size={16} strokeWidth={1.75} />
      {:else}
        <PanelLeftClose size={16} strokeWidth={1.75} />
      {/if}
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: 240px;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    transition: width 0.15s ease;
    overflow: hidden;
    flex-shrink: 0;
  }

  .sidebar.collapsed { width: 56px; }

  .sidebar-header {
    padding: 16px 16px 12px;
    border-bottom: 1px solid var(--border);
  }

  .logo { display: flex; align-items: center; gap: 8px; }
  .logo-img { width: 24px; height: 24px; }
  .logo-text { font-size: 0.95rem; font-weight: 700; color: var(--text-primary); white-space: nowrap; }

  .new-memory-btn {
    margin: 12px 16px;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .new-memory-btn:hover { opacity: 0.85; }
  kbd {
    margin-left: auto;
    font-size: 0.7rem;
    padding: 1px 4px;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
    font-family: var(--font-mono);
  }

  .nav { flex: 1; padding: 8px 0; display: flex; flex-direction: column; }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    text-align: left;
    width: 100%;
  }
  .nav-item:hover { background: var(--bg-hover); color: var(--text-primary); }
  .nav-item.active {
    background: var(--bg-hover);
    color: var(--accent);
    border-left: 2px solid var(--accent);
    padding-left: 14px;
  }
  .nav-item.active :global(svg) { stroke: var(--accent); }
  .nav-icon {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .nav-label { white-space: nowrap; flex: 1; }

  .sidebar-footer { padding: 8px 16px; border-top: 1px solid var(--border); }

  .nav-bottom { padding: 8px 0; border-top: 1px solid var(--border); }

  .server-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .server-status.online { color: var(--green); }
  .server-status.offline { color: var(--text-muted); }
  .server-status-collapsed {
    display: flex;
    justify-content: center;
    padding: 6px;
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--text-muted);
  }
  .online .status-dot {
    background: var(--green);
    box-shadow: 0 0 6px rgba(166, 227, 161, 0.5);
    animation: pulse 2s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .collapse-btn {
    width: 100%;
    padding: 6px 8px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background 0.1s;
  }
  .collapse-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }

  .sidebar.collapsed .sidebar-header,
  .sidebar.collapsed .new-memory-btn { display: none; }
  .sidebar.collapsed .nav-item { justify-content: center; padding: 10px; }
  .sidebar.collapsed .nav-bottom .nav-item { justify-content: center; padding: 10px; }
  .sidebar.collapsed .nav-item.active { padding-left: 10px; }
</style>
