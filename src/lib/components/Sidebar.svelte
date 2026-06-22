<script lang="ts">
  import type { View } from '../ts/types';

  interface Props {
    activeView: View;
    collapsed: boolean;
    onnavigate: (view: View) => void;
    onnewmemory: () => void;
    oncollapse: () => void;
  }

  let { activeView, collapsed, onnavigate, onnewmemory, oncollapse }: Props = $props();

  const navItems: { view: View; label: string; icon: string }[] = [
    { view: 'dashboard', label: 'Dashboard', icon: '◧' },
    { view: 'memories', label: 'Memories', icon: '☰' },
    { view: 'namespaces', label: 'Namespaces', icon: '◫' },
    { view: 'graph', label: 'Graph', icon: '◉' },
    { view: 'rooms', label: 'Rooms', icon: '▣' },
  ];

  const bottomItems: { view: View; label: string; icon: string }[] = [
    { view: 'settings', label: 'Settings', icon: '⚙' },
  ];
</script>

<aside class="sidebar" class:collapsed>
  {#if !collapsed}
    <div class="sidebar-header">
      <div class="logo">
        <span class="logo-icon">◆</span>
        <span class="logo-text">Codecora Hub</span>
      </div>
    </div>

    <button class="new-memory-btn" onclick={onnewmemory}>
      <span class="btn-icon">+</span>
      <span>New Memory</span>
      <kbd>Ctrl+N</kbd>
    </button>
  {/if}

  <nav class="nav">
    {#each navItems as item}
      <button
        class="nav-item"
        class:active={activeView === item.view}
        onclick={() => onnavigate(item.view)}
        title={collapsed ? item.label : ''}
      >
        <span class="nav-icon">{item.icon}</span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="nav-bottom">
    {#each bottomItems as item}
      <button
        class="nav-item"
        class:active={activeView === item.view}
        onclick={() => onnavigate(item.view)}
        title={collapsed ? item.label : ''}
      >
        <span class="nav-icon">{item.icon}</span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="sidebar-footer">
    <button class="collapse-btn" onclick={oncollapse} title={collapsed ? 'Expand (Ctrl+B)' : 'Collapse (Ctrl+B)'}>
      {collapsed ? '▶' : '◀'}
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
  .logo-icon { font-size: 1.3rem; color: var(--accent); }
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
  .btn-icon { font-size: 1.1rem; line-height: 1; }
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
    transition: background 0.1s;
    text-align: left;
    width: 100%;
  }
  .nav-item:hover { background: var(--bg-hover); }
  .nav-item.active { background: var(--bg-hover); color: var(--accent); border-left: 2px solid var(--accent); }
  .nav-icon { font-size: 1rem; width: 20px; text-align: center; flex-shrink: 0; }
  .nav-label { white-space: nowrap; flex: 1; }

  .sidebar-footer { padding: 8px 16px; border-top: 1px solid var(--border); }

  .nav-bottom { padding: 8px 0; border-top: 1px solid var(--border); }

  .nav-bottom .nav-item { /* same styling as main nav */ }

  .collapse-btn {
    width: 100%;
    padding: 4px 8px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.8rem;
    text-align: center;
    border-radius: 4px;
    transition: background 0.1s;
  }
  .collapse-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }

  .sidebar.collapsed .sidebar-header,
  .sidebar.collapsed .new-memory-btn { display: none; }
  .sidebar.collapsed .nav-item { justify-content: center; padding: 8px; }
  .sidebar.collapsed .nav-bottom .nav-item { justify-content: center; padding: 8px; }
</style>
