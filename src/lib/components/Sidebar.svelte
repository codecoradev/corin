<script lang="ts">
  import type { View } from '../ts/types';
  import { utekeServer } from '../ts/ipc';
  import { theme } from '../stores/theme.svelte';
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
    HeartPulse,
    Sun,
    Moon,
  } from 'lucide-svelte';

  interface Props {
    activeView: View;
    collapsed: boolean;
    onnavigate: (view: View) => void;
    onnewmemory: () => void;
    oncollapse: () => void;
  }

  let { activeView, collapsed, onnavigate, onnewmemory, oncollapse }: Props = $props();

  // Uteke server status — always visible in the rail
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
    { view: 'lifecycle', label: 'Lifecycle', icon: HeartPulse },
    // Tools hidden from nav — features being reworked (#244). Route still reachable.
    // { view: 'tools', label: 'Tools', icon: Wrench },
  ];

  type IconComp = typeof LayoutDashboard;
  const iconSize = 18;
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-header" class:hidden={collapsed}>
    {#if !collapsed}
      <div class="logo">
        <img src="/corin-logo.png" alt="CorIn" class="logo-img" />
        <span class="logo-text">CorIn</span>
      </div>
    {/if}
    {#if collapsed}
      <button class="rail-btn new-memory-rail" onclick={onnewmemory} title="New Memory (Ctrl+N)" aria-label="New Memory">
        <Plus size={18} strokeWidth={2.25} />
      </button>
    {:else}
      <button class="new-memory-btn" onclick={onnewmemory}>
        <Plus size={16} strokeWidth={2.5} />
        <span>New Memory</span>
        <kbd>Ctrl+N</kbd>
      </button>
    {/if}
  </div>

  <nav class="nav" aria-label="Primary">
    {#each navItems as item (item.view)}
      <button
        class="nav-item"
        class:active={activeView === item.view}
        onclick={() => onnavigate(item.view)}
        title={collapsed ? item.label : ''}
        aria-label={item.label}
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
      <div
        class="server-status-collapsed"
        class:online={serverOnline}
        title={serverOnline ? 'Semantic search active' : 'uteke-serve offline'}
      >
        <span class="status-dot"></span>
      </div>
    {/if}

    <button
      class="rail-btn"
      onclick={() => theme.toggle()}
      title={theme.current === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
      aria-label="Toggle theme"
    >
      {#if theme.current === 'dark'}
        <Sun size={16} strokeWidth={1.75} />
      {:else}
        <Moon size={16} strokeWidth={1.75} />
      {/if}
      {#if !collapsed}
        <span class="rail-btn-label">{theme.current === 'dark' ? 'Light theme' : 'Dark theme'}</span>
      {/if}
    </button>

    <button
      class="nav-item"
      class:active={activeView === 'settings'}
      onclick={() => onnavigate('settings')}
      title={collapsed ? 'Settings' : ''}
      aria-label="Settings"
    >
      <span class="nav-icon">
        <Settings size={iconSize} strokeWidth={1.75} />
      </span>
      {#if !collapsed}
        <span class="nav-label">Settings</span>
      {/if}
    </button>

    <button class="rail-btn" onclick={oncollapse} title={collapsed ? 'Expand (Ctrl+B)' : 'Collapse (Ctrl+B)'} aria-label="Toggle sidebar">
      {#if collapsed}
        <PanelLeftOpen size={16} strokeWidth={1.75} />
      {:else}
        <PanelLeftClose size={16} strokeWidth={1.75} />
      {/if}
      {#if !collapsed}
        <span class="rail-btn-label">Collapse</span>
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
    transition: width 0.15s var(--ease-out);
    overflow: hidden;
    flex-shrink: 0;
  }

  .sidebar.collapsed { width: 56px; }

  .sidebar-header {
    padding: 14px 16px 4px;
  }
  .sidebar-header.hidden {
    padding: 12px 0 4px;
    display: flex;
    justify-content: center;
  }

  .logo { display: flex; align-items: center; gap: 8px; }
  .logo-img { width: 24px; height: 24px; }
  .logo-text { font-size: 0.95rem; font-weight: 700; color: var(--text-primary); white-space: nowrap; }

  /* Compact rail CTA — mirrors mockup A (teal tile, 2+ changes on hover) */
  .new-memory-rail {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: opacity 0.15s var(--ease-out), transform 0.15s var(--ease-out);
  }
  .new-memory-rail:hover { opacity: 0.85; }
  .new-memory-rail:active { transform: scale(0.98); }

  .new-memory-btn {
    margin-top: 10px;
    width: 100%;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s var(--ease-out);
  }
  .new-memory-btn:hover { opacity: 0.85; }
  kbd {
    margin-left: auto;
    font-size: 0.7rem;
    padding: 1px 4px;
    border: 1px solid currentColor;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    opacity: 0.75;
  }

  .nav { flex: 1; padding: 10px 8px; display: flex; flex-direction: column; gap: 2px; }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.15s var(--ease-out), color 0.15s var(--ease-out);
    text-align: left;
    width: 100%;
  }
  .nav-item:hover { background: var(--bg-hover); color: var(--text-primary); }
  /* Active = 2+ visual changes (ui-standards): tint bg + accent color + icon fill accent + left indicator */
  .nav-item.active {
    background: var(--color-teal-bg);
    color: var(--accent);
    box-shadow: inset 2px 0 0 var(--accent);
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

  .nav-bottom {
    padding: 8px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .server-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .server-status.online { color: var(--green); }
  .server-status.offline { color: var(--text-muted); }
  .server-status-collapsed {
    display: flex;
    justify-content: center;
    padding: 6px 0;
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
    animation: pulse 2s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  /* Rail utility buttons (theme toggle, collapse) */
  .rail-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.15s var(--ease-out), color 0.15s var(--ease-out);
  }
  .rail-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .rail-btn-label { font-size: 0.8rem; white-space: nowrap; }

  .sidebar.collapsed .nav-item { justify-content: center; padding: 10px 0; width: auto; }
  .sidebar.collapsed .nav { padding: 10px 8px; align-items: center; }
  .sidebar.collapsed .nav-bottom { padding: 8px 8px; align-items: center; }
</style>
