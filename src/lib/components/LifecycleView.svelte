<script lang="ts">
  import {
    lifecycleStatus,
    lifecycleCycle,
    lifecyclePromote,
    lifecycleDeprecated,
    findOrphans,
  } from '../ts/ipc';
  import type {
    LifecycleStatus,
    LifecycleCycleResult,
    OrphanMemory,
    DeprecatedMemoryInfo,
  } from '../ts/types';
  import { Spinner, Button, ConfirmDialog } from '../ui';
  import { toastStore } from '../ui';
  import ConsolidatePanel from './ConsolidatePanel.svelte';
  import {
    HeartPulse,
    RefreshCw,
    Trash2,
    RotateCcw,
    AlertTriangle,
    CheckCircle2,
  } from 'lucide-svelte';

  interface Props {
    namespace: string | null;
    /** Open the memory detail panel for an item in the recycle bin. */
    onmemoryclick?: (id: string) => void;
  }

  let { namespace, onmemoryclick }: Props = $props();

  // ─── State ─────────────────────────────────────────────────────────
  let status = $state<LifecycleStatus | null>(null);
  let deprecatedItems = $state<DeprecatedMemoryInfo[]>([]);
  let orphans = $state<OrphanMemory[]>([]);
  let loading = $state(true);
  let cycling = $state(false);
  let promotingIds = $state<Set<string>>(new Set());
  let lastCycleResult = $state<LifecycleCycleResult | null>(null);
  let showConfirmCycle = $state(false);
  let error = $state<string | null>(null);

  // ─── Data loading ──────────────────────────────────────────────────
  async function loadData() {
    loading = true;
    error = null;
    try {
      const ns = namespace ?? undefined;
      const [s, d, o] = await Promise.all([
        lifecycleStatus(ns).catch((e) => {
          throw e;
        }),
        lifecycleDeprecated(ns, 100).catch(() => ({ deprecated: [], count: 0 })),
        findOrphans(ns).catch(() => [] as OrphanMemory[]),
      ]);
      status = s;
      deprecatedItems = d.deprecated;
      orphans = o;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      // Graceful: show zeros if server doesn't support lifecycle yet
      if (String(error).includes('404') || String(error).includes('not found')) {
        status = { active: 0, deprecated: 0, pruned: 0 };
        error = 'Uteke server does not support lifecycle endpoints yet. Requires uteke ≥ 0.13.0.';
      }
    } finally {
      loading = false;
    }
  }

  // Load on mount + reload when namespace changes
  $effect(() => {
    namespace;
    loadData();
  });

  // ─── Actions ───────────────────────────────────────────────────────
  async function runCycle() {
    showConfirmCycle = false;
    cycling = true;
    try {
      const result = await lifecycleCycle(namespace ?? undefined);
      lastCycleResult = result;
      toastStore.success(
        `Lifecycle cycle complete: ${result.deprecated} deprecated, ${result.pruned} pruned`
      );
      await loadData();
    } catch (e) {
      toastStore.error(`Cycle failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      cycling = false;
    }
  }

  async function promoteMemory(id: string) {
    promotingIds = new Set([...promotingIds, id]);
    try {
      await lifecyclePromote(id);
      toastStore.success('Memory restored to active');
      await loadData();
    } catch (e) {
      toastStore.error(`Restore failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      const next = new Set(promotingIds);
      next.delete(id);
      promotingIds = next;
    }
  }

  // ─── Helpers ───────────────────────────────────────────────────────
  function totalMemories(): number {
    if (!status) return 0;
    return status.active + status.deprecated;
  }

  function activePercent(): number {
    const total = totalMemories();
    if (total === 0 || !status) return 100;
    return Math.round((status.active / total) * 100);
  }
</script>

<div class="lifecycle-view">
  <header class="view-header">
    <div class="header-left">
      <HeartPulse size={22} strokeWidth={1.75} />
      <div>
        <h1>Memory Lifecycle</h1>
        <p class="subtitle">
          {#if namespace}
            Namespace: <code>{namespace}</code>
          {:else}
            All namespaces
          {/if}
        </p>
      </div>
    </div>
    <div class="header-actions">
      <Button variant="secondary" onclick={() => loadData()} disabled={loading}>
        <span class:spinning={loading}><RefreshCw size={15} strokeWidth={2} /></span>
        <span>Refresh</span>
      </Button>
      <Button variant="primary" onclick={() => (showConfirmCycle = true)} disabled={cycling || loading}>
        <span class:spinning={cycling}><RotateCcw size={15} strokeWidth={2} /></span>
        <span>Run Cycle</span>
      </Button>
    </div>
  </header>

  {#if loading}
    <div class="loading-state">
      <Spinner size={32} />
      <p>Loading lifecycle status…</p>
    </div>
  {:else if error}
    <div class="error-banner">
      <AlertTriangle size={18} />
      <span>{error}</span>
    </div>
  {:else}
    <!-- ─── Status Cards ─────────────────────────────────────────── -->
    <div class="status-grid">
      <div class="status-card card-active">
        <div class="card-icon">
          <CheckCircle2 size={20} strokeWidth={1.75} />
        </div>
        <div class="card-body">
          <span class="card-value">{status?.active ?? 0}</span>
          <span class="card-label">Active</span>
        </div>
      </div>
      <div class="status-card card-deprecated">
        <div class="card-icon">
          <AlertTriangle size={20} strokeWidth={1.75} />
        </div>
        <div class="card-body">
          <span class="card-value">{status?.deprecated ?? 0}</span>
          <span class="card-label">Deprecated</span>
        </div>
      </div>
    </div>

    <!-- ─── Health Bar ──────────────────────────────────────────── -->
    <div class="health-bar-section">
      <div class="health-bar-header">
        <span class="health-label">Memory Health</span>
        <span class="health-percent">{activePercent()}% active</span>
      </div>
      <div class="health-bar">
        <div
          class="health-segment seg-active"
          style="width: {status && totalMemories() > 0
            ? (status.active / totalMemories()) * 100
            : 0}%"
          title="{status?.active ?? 0} active"
        ></div>
        <div
          class="health-segment seg-deprecated"
          style="width: {status && totalMemories() > 0
            ? (status.deprecated / totalMemories()) * 100
            : 0}%"
          title="{status?.deprecated ?? 0} deprecated"
        ></div>
      </div>
    </div>

    <!-- ─── Last Cycle Result ───────────────────────────────────── -->
    {#if lastCycleResult}
      <div class="cycle-result">
        <RotateCcw size={16} strokeWidth={1.75} />
        <span>
          Last cycle: <strong>{lastCycleResult.deprecated}</strong> deprecated,
          <strong>{lastCycleResult.pruned}</strong> pruned,
          <strong>{lastCycleResult.skipped}</strong> skipped
        </span>
      </div>
    {/if}

    <!-- ─── Recycle Bin (Deprecated) ────────────────────── -->
    <section class="orphans-section">
      <div class="section-header">
        <Trash2 size={18} strokeWidth={1.75} />
        <h2>Recycle Bin</h2>
        <span class="badge">{deprecatedItems.length}</span>
      </div>
      <p class="section-desc">
        Deprecated memories — hidden from search, kept until pruned past their TTL. Restore to
        make them active again.
      </p>

      {#if deprecatedItems.length === 0}
        <div class="empty-state">
          <CheckCircle2 size={28} strokeWidth={1.5} />
          <p>Recycle bin is empty.</p>
          <p class="empty-hint">Memories deprecated by a cycle land here — restore them any time before their TTL expires.</p>
        </div>
      {:else}
        <div class="orphans-list">
          {#each deprecatedItems as item (item.id)}
            <div class="orphan-item">
              <div
                class="orphan-content clickable"
                role="button"
                tabindex="0"
                title="View details"
                onclick={() => onmemoryclick?.(item.id)}
                onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onmemoryclick?.(item.id)}
              >
                <p class="orphan-text">{item.content.slice(0, 200)}</p>
                <div class="orphan-meta">
                  <code class="orphan-id">{item.id.slice(0, 12)}…</code>
                  {#if item.namespace}
                    <span class="tag tag-ns">{item.namespace}</span>
                  {/if}
                  {#if item.deprecate_reason}
                    <span class="tag" title="Why this was deprecated">{item.deprecate_reason}</span>
                  {/if}
                  {#each item.tags.slice(0, 3) as tag}
                    <span class="tag">{tag}</span>
                  {/each}
                </div>
              </div>
              <Button
                size="sm"
                variant="primary"
                onclick={() => promoteMemory(item.id)}
                disabled={promotingIds.has(item.id)}
              >
                {#if promotingIds.has(item.id)}
                  <span class="spinning"><RefreshCw size={13} /></span>
                  <span>Restoring…</span>
                {:else}
                  <RotateCcw size={13} />
                  <span>Restore</span>
                {/if}
              </Button>
            </div>
          {/each}
        </div>
        {#if deprecatedItems.length >= 100}
          <p class="section-desc">
            Showing the 100 most recent — older entries still count toward the total.
          </p>
        {/if}
      {/if}
    </section>

    <!-- ─── Consolidate Panel (#230) ──────────────────────── -->
    <ConsolidatePanel {namespace} />
  {/if}

  <!-- ─── Confirm Cycle Modal ──────────────────────────────────────── -->
  <ConfirmDialog
    open={showConfirmCycle}
    title="Run Lifecycle Cycle?"
    message="This will deprecate memories that haven't been accessed in a while, and permanently prune memories that have been deprecated past their TTL."
    confirmLabel="Run Cycle"
    onconfirm={runCycle}
    oncancel={() => (showConfirmCycle = false)}
  >
    {#if status && status.deprecated > 0}
      <p class="confirm-warn">
        <strong>{status.deprecated}</strong> memories are currently deprecated.
        Some may be pruned if their TTL has expired.
      </p>
    {/if}
  </ConfirmDialog>
</div>

<style>
  .lifecycle-view {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    padding: 1.5rem 2rem;
    max-width: 900px;
    margin: 0 auto;
  }

  /* ─── Header ────────────────────────────────────────────────────── */
  .view-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.75rem;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .header-left h1 {
    font-size: 1.35rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0.15rem 0 0 0;
  }

  .subtitle code {
    background: var(--bg-tertiary);
    padding: 0.1rem 0.35rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  /* ─── Loading & Error ───────────────────────────────────────────── */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 0;
    color: var(--text-muted);
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.85rem 1rem;
    background: rgba(243, 139, 168, 0.08);
    border: 1px solid rgba(243, 139, 168, 0.25);
    border-radius: var(--radius-md);
    color: var(--color-red);
    font-size: 0.85rem;
  }

  /* ─── Status Cards ──────────────────────────────────────────────── */
  .status-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.85rem;
    margin-bottom: 1.5rem;
  }

  .status-card {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    padding: 1.1rem 1.25rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }

  .card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    flex-shrink: 0;
  }

  .card-active .card-icon {
    background: rgba(166, 227, 161, 0.1);
    color: var(--color-green);
  }

  .card-deprecated .card-icon {
    background: rgba(249, 226, 175, 0.1);
    color: var(--color-yellow);
  }

  .card-body {
    display: flex;
    flex-direction: column;
  }

  .card-value {
    font-size: 1.6rem;
    font-weight: 700;
    line-height: 1.1;
    color: var(--text-primary);
  }

  .card-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-top: 0.1rem;
  }

  /* ─── Health Bar ────────────────────────────────────────────────── */
  .health-bar-section {
    margin-bottom: 1.5rem;
  }

  .health-bar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.4rem;
  }

  .health-label {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .health-percent {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .health-bar {
    display: flex;
    height: 8px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg-hover);
  }

  .health-segment {
    transition: width 0.4s ease;
    min-width: 0;
  }

  .seg-active {
    background: var(--color-green);
  }

  .seg-deprecated {
    background: var(--color-yellow);
  }

  /* ─── Cycle Result ──────────────────────────────────────────────── */
  .cycle-result {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.85rem;
    background: rgba(137, 180, 250, 0.06);
    border: 1px solid rgba(137, 180, 250, 0.15);
    border-radius: var(--radius-md);
    font-size: 0.82rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  /* ─── Orphans Section ───────────────────────────────────────────── */
  .orphans-section {
    margin-top: 1.5rem;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.3rem;
  }

  .section-header h2 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 0.35rem;
    border-radius: var(--radius-lg);
    background: var(--bg-hover);
    color: var(--text-muted);
    font-size: 0.72rem;
    font-weight: 600;
  }

  .section-desc {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0.1rem 0 1rem 1.7rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    padding: 2rem 0;
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  .empty-state .empty-hint {
    font-size: 0.78rem;
    opacity: 0.7;
    max-width: 420px;
    margin: 0;
  }

  .orphans-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .orphan-item {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.85rem;
    padding: 0.75rem 1rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .orphan-content {
    flex: 1;
    min-width: 0;
  }

  /* Clickable content area → opens the detail panel */
  .orphan-content.clickable {
    cursor: pointer;
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    margin: -2px -6px;
    transition: background-color 0.12s var(--ease-out);
  }

  .orphan-content.clickable:hover,
  .orphan-content.clickable:focus-visible {
    background: var(--bg-hover);
    outline: none;
  }

  .orphan-text {
    font-size: 0.85rem;
    color: var(--text-primary);
    margin: 0 0 0.4rem 0;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .orphan-meta {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .orphan-id {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .tag {
    display: inline-block;
    padding: 0.1rem 0.4rem;
    border-radius: var(--radius-sm);
    background: var(--bg-hover);
    color: var(--text-secondary);
    font-size: 0.7rem;
  }

  .tag-ns {
    background: rgba(148, 226, 213, 0.1);
    color: var(--color-teal);
  }


  /* ─── Confirm dialog extras ─────────────────────────────────────── */
  .confirm-warn {
    font-size: 0.8rem;
    color: var(--yellow);
    background: rgba(249, 226, 175, 0.06);
    border: 1px solid rgba(249, 226, 175, 0.15);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.75rem;
    margin: 0;
    text-align: left;
  }

  /* ─── Animations ────────────────────────────────────────────────── */
  .spinning {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
