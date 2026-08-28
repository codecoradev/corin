<script lang="ts">
  import { utekeContext } from '../ts/ipc';
  import { toastStore, Button } from '../ui';
  import {
    BrainCircuit,
    Check,
    Copy,
    RefreshCw,
    Terminal,
    Hash,
    FileText,
    Rows3,
} from 'lucide-svelte';

  interface Props {
    namespace: string | null;
    namespaces: string[];
  }

  let { namespace, namespaces = [] }: Props = $props();

  // ─── State ─────────────────────────────────────────────────────────
  let selectedNs = $state<string>('');
  let contextText = $state<string | null>(null);
  let loading = $state(false);
  let copied = $state(false);
  let error = $state<string | null>(null);

  // ─── Actions ───────────────────────────────────────────────────────
  async function loadContext() {
    loading = true;
    error = null;
    try {
      const ctx = await utekeContext(selectedNs || undefined);
      contextText = ctx;
      if (!ctx) {
        error = 'Server returned an empty context.';
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      contextText = null;
    } finally {
      loading = false;
    }
  }

  async function copyContext() {
    if (!contextText) return;
    try {
      await navigator.clipboard.writeText(contextText);
      copied = true;
      setTimeout(() => (copied = false), 1600);
    } catch {
      toastStore.error('Copy failed — clipboard unavailable');
    }
  }

  // On mount: default selector to the app's active namespace, auto-load once.
  let loadedOnce = $state(false);
  $effect(() => {
    if (!selectedNs) {
      selectedNs = namespace ?? '';
    }
    if (!loadedOnce) {
      loadedOnce = true;
      loadContext();
    }
  });

  // ─── Helpers ───────────────────────────────────────────────────────
  function tokenEstimate(text: string | null): number {
    if (!text) return 0;
    return Math.ceil(text.length / 4);
  }

  function memoryCount(text: string | null): number | null {
    if (!text) return null;
    const match = text.match(/Project memory:\s+(\d+)\s+memor/);
    return match ? parseInt(match[1], 10) : null;
  }

  function lineCount(text: string | null): number {
    if (!text) return 0;
    return text.split('\n').length;
  }
</script>

<section class="context-section">
  <div class="section-header">
    <BrainCircuit size={18} strokeWidth={1.75} />
    <h2>Context Preview</h2>
    {#if contextText}
      <span class="badge">{selectedNs || 'default'}</span>
    {/if}
    <div class="header-actions">
      <select
        class="ns-select"
        bind:value={selectedNs}
        disabled={loading}
        aria-label="Namespace"
        onchange={() => loadContext()}
      >
        <option value="">default</option>
        {#each namespaces.filter((ns) => ns && ns !== 'default') as ns}
          <option value={ns}>{ns}</option>
        {/each}
      </select>
      <Button size="sm" variant="secondary" onclick={loadContext} disabled={loading}>
        <span class:spinning={loading}><RefreshCw size={13} /></span>
        <span>Refresh</span>
      </Button>
      <Button
        size="sm"
        variant="secondary"
        onclick={copyContext}
        disabled={!contextText || loading}
        title="Copy context to clipboard"
      >
        {#if copied}
          <Check size={13} />
          <span>Copied</span>
        {:else}
          <Copy size={13} />
          <span>Copy</span>
        {/if}
      </Button>
    </div>
  </div>
  <p class="section-desc">
    Exactly what your agent's LLM sees when uteke injects memory context.
  </p>

  {#if error}
    <div class="error-banner">
      <Terminal size={16} />
      <span>{error}</span>
      <Button size="sm" variant="secondary" onclick={loadContext} disabled={loading}>
        Retry
      </Button>
    </div>
  {:else if loading && !contextText}
    <div class="loading-state">
      <span class="spinning"><RefreshCw size={20} /></span>
      <p>Building context…</p>
    </div>
  {:else if contextText}
    <!-- ─── Summary cards (LifecycleView pattern) ───────────────── -->
    <div class="status-grid">
      <div class="status-card card-tokens">
        <div class="card-icon">
          <Hash size={20} strokeWidth={1.75} />
        </div>
        <div class="card-body">
          <span class="card-value">~{tokenEstimate(contextText)}</span>
          <span class="card-label">Tokens (est.)</span>
        </div>
      </div>
      <div class="status-card card-memories">
        <div class="card-icon">
          <BrainCircuit size={20} strokeWidth={1.75} />
        </div>
        <div class="card-body">
          <span class="card-value">{memoryCount(contextText) ?? '—'}</span>
          <span class="card-label">Memories</span>
        </div>
      </div>
      <div class="status-card card-lines">
        <div class="card-icon">
          <Rows3 size={20} strokeWidth={1.75} />
        </div>
        <div class="card-body">
          <span class="card-value">{lineCount(contextText)}</span>
          <span class="card-label">Lines</span>
        </div>
      </div>
    </div>

    <!-- ─── The exact context string ────────────────────────────── -->
    <pre class="context-block">{contextText}</pre>
  {:else}
    <div class="empty-state">
      <FileText size={26} strokeWidth={1.5} />
      <p>No context loaded.</p>
      <Button size="sm" variant="primary" onclick={loadContext} disabled={loading}>
        Load Context
      </Button>
    </div>
  {/if}
</section>

<style>
  .context-section {
    margin-top: 1.75rem;
  }

  /* ─── Section header (LifecycleView pattern) ────────────────────── */
  .section-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.3rem;
    color: var(--text-primary);
  }

  .section-header h2 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-left: auto;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 0.45rem;
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

  .ns-select {
    font-size: 0.78rem;
    padding: 0.3rem 0.5rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    max-width: 160px;
    cursor: pointer;
  }

  /* ─── Status cards (LifecycleView pattern) ──────────────────────── */
  .status-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.85rem;
    margin-bottom: 1rem;
  }

  .status-card {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    padding: 1rem 1.15rem;
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

  .card-tokens .card-icon {
    background: rgba(137, 180, 250, 0.1);
    color: var(--color-blue);
  }

  .card-memories .card-icon {
    background: rgba(166, 227, 161, 0.1);
    color: var(--color-green);
  }

  .card-lines .card-icon {
    background: rgba(203, 166, 247, 0.1);
    color: var(--color-mauve);
  }

  .card-body {
    display: flex;
    flex-direction: column;
  }

  .card-value {
    font-size: 1.4rem;
    font-weight: 700;
    line-height: 1.1;
    color: var(--text-primary);
  }

  .card-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-top: 0.1rem;
  }

  /* ─── Context block ─────────────────────────────────────────────── */
  .context-block {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.76rem;
    line-height: 1.55;
    color: var(--text-primary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 1rem 1.15rem;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 340px;
    overflow-y: auto;
  }

  /* ─── States ────────────────────────────────────────────────────── */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    padding: 2rem 0;
    color: var(--text-muted);
    font-size: 0.85rem;
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

  .error-banner :global(.btn) {
    margin-left: auto;
  }

  .spinning {
    display: inline-flex;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
