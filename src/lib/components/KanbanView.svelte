<script lang="ts">
  import { kanban } from '../ts/ipc';
  import type { KanbanBoard, KanbanTask } from '../ts/types';
  import KanbanCard from './KanbanCard.svelte';
  import KanbanDrawer from './KanbanDrawer.svelte';
  import KanbanCreateModal from './KanbanCreateModal.svelte';

  // Board column config
  const COLUMNS = [
    { name: 'triage', label: 'Triage' },
    { name: 'todo', label: 'Todo' },
    { name: 'scheduled', label: 'Scheduled' },
    { name: 'ready', label: 'Ready' },
    { name: 'running', label: 'Running' },
    { name: 'blocked', label: 'Blocked' },
    { name: 'review', label: 'Review' },
    { name: 'done', label: 'Done' },
  ];

  const columnOrder = COLUMNS.map(c => c.name);

  // State
  let board = $state<KanbanBoard | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let available = $state(false);
  let selectedTaskId = $state<string | null>(null);
  let showCreate = $state(false);
  let filterTenant = $state<string | null>(null);
  let searchFilter = $state('');
  let lastRefresh = $state(0);

  // Derived: columns sorted by standard order
  let sortedColumns = $derived(() => {
    if (!board) return [];
    return [...board.columns].sort((a, b) => {
      const ai = columnOrder.indexOf(a.name);
      const bi = columnOrder.indexOf(b.name);
      return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
    });
  });

  // Derived: filter tasks
  function filterTasks(tasks: KanbanTask[]): KanbanTask[] {
    let filtered = tasks;
    if (searchFilter) {
      const q = searchFilter.toLowerCase();
      filtered = filtered.filter(
        t => t.title.toLowerCase().includes(q) ||
          t.assignee?.toLowerCase().includes(q) ||
          t.id.toLowerCase().includes(q)
      );
    }
    return filtered;
  }

  // Check availability + load board
  async function init() {
    loading = true;
    error = null;
    try {
      available = await kanban.available();
      if (available) {
        await loadBoard();
      }
    } catch (e) {
      error = `Connection failed: ${e}`;
      available = false;
    } finally {
      loading = false;
    }
  }

  async function loadBoard() {
    if (!available) return;
    try {
      board = await kanban.board({
        tenant: filterTenant || undefined,
      });
    } catch (e) {
      error = String(e);
    }
  }

  // Auto-refresh every 30s
  $effect(() => {
    init();
    const interval = setInterval(() => {
      if (available) loadBoard();
    }, 30_000);
    return () => clearInterval(interval);
  });

  // Re-load when tenant filter changes
  $effect(() => {
    filterTenant; // track
    if (available) loadBoard();
  });

  function openTask(task: KanbanTask) {
    selectedTaskId = task.id;
  }

  function closeDrawer() {
    selectedTaskId = null;
  }

  function handleRefresh() {
    lastRefresh++;
    loadBoard();
  }

  // Total task count
  let totalTasks = $derived(
    board?.columns.reduce((sum, col) => sum + col.tasks.length, 0) ?? 0
  );
</script>

<div class="kanban-view">
  <!-- Header bar -->
  <div class="kanban-header">
    <h2 class="kanban-title">
      Kanban
      {#if board}
        <span class="task-count">{totalTasks} tasks</span>
      {/if}
    </h2>

    <div class="kanban-actions">
      {#if board}
        <input
          class="search-input"
          type="text"
          placeholder="Filter tasks..."
          bind:value={searchFilter}
        />

        {#if board.tenants.length > 1}
          <select class="tenant-select" bind:value={filterTenant}>
            <option value="">All tenants</option>
            {#each board.tenants as t}
              <option value={t}>{t}</option>
            {/each}
          </select>
        {/if}
      {/if}

      <button class="btn-refresh" onclick={handleRefresh} title="Refresh">
        ↻
      </button>
      <button class="btn-create" onclick={() => showCreate = true} title="New task (Ctrl+T)">
        + New Task
      </button>
    </div>
  </div>

  <!-- Content -->
  {#if loading}
    <div class="kanban-loading">
      <div class="spinner"></div>
      <span>Connecting to Hermes dashboard...</span>
    </div>
  {:else if !available}
    <div class="kanban-empty">
      <span class="empty-icon">📋</span>
      <h3>Hermes Dashboard Not Connected</h3>
      <p>Start the Hermes dashboard to view the kanban board.</p>
      <code>hermes dashboard</code>
    </div>
  {:else if error}
    <div class="kanban-empty">
      <span class="empty-icon">⚠</span>
      <h3>Error</h3>
      <p>{error}</p>
      <button class="btn-retry" onclick={init}>Retry</button>
    </div>
  {:else if board && sortedColumns().length > 0}
    <div class="board">
      {#each sortedColumns() as col (col.name)}
        <div class="column" class:column-empty={filterTasks(col.tasks).length === 0}>
          <div class="column-header">
            <span class="column-count">{filterTasks(col.tasks).length}</span>
            <span class="column-name">{col.name}</span>
          </div>
          <div class="column-cards">
            {#each filterTasks(col.tasks) as task (task.id)}
              {#key lastRefresh}<!-- force re-render on refresh -->
                <KanbanCard {task} onclick={openTask} />
              {/key}
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Task drawer -->
{#if selectedTaskId}
  <KanbanDrawer
    taskId={selectedTaskId}
    onclose={closeDrawer}
    onrefresh={handleRefresh}
  />
{/if}

<!-- Create modal -->
{#if showCreate}
  <KanbanCreateModal
    onclose={() => showCreate = false}
    oncreated={handleRefresh}
  />
{/if}

<style>
  .kanban-view {
    height: 100%;
    display: flex;
    flex-direction: column;
    font-family: var(--font-sans);
  }

  .kanban-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
    flex-wrap: wrap;
  }

  .kanban-title {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
  }

  .task-count {
    font-size: 0.75rem;
    font-weight: 400;
    color: var(--text-muted);
    background: var(--bg-hover);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .kanban-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search-input {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 0.85rem;
    padding: 6px 10px;
    width: 180px;
  }
  .search-input:focus { outline: none; border-color: var(--accent); }
  .search-input::placeholder { color: var(--text-muted); }

  .tenant-select {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: 0.8rem;
    padding: 5px 8px;
  }
  .tenant-select:focus { outline: none; border-color: var(--accent); }

  .btn-refresh {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px 8px;
    transition: background 0.15s;
  }
  .btn-refresh:hover { background: var(--bg-hover); }

  .btn-create {
    padding: 6px 14px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
    white-space: nowrap;
  }
  .btn-create:hover { opacity: 0.85; }

  .kanban-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .kanban-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 8px;
    color: var(--text-muted);
    text-align: center;
  }

  .empty-icon {
    font-size: 2rem;
  }

  .kanban-empty h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .kanban-empty p {
    margin: 0;
    font-size: 0.85rem;
  }

  .kanban-empty code {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    background: var(--bg-primary);
    padding: 4px 12px;
    border-radius: var(--radius);
    margin-top: 8px;
  }

  .btn-retry {
    margin-top: 8px;
    padding: 6px 16px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  /* Board layout — horizontal scrolling columns */
  .board {
    flex: 1;
    display: flex;
    gap: 0;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 12px 0;
  }

  .column {
    flex: 0 0 280px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    padding: 0 8px;
    min-height: 0;
  }
  .column:last-child { border-right: none; }
  .column-empty { opacity: 0.6; }

  .column-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px 8px;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .column-count {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-hover);
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .column-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    white-space: nowrap;
  }

  .column-cards {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-right: 4px;
  }
</style>
