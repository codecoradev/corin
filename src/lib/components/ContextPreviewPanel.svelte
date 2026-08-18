<script lang="ts">
  import { utekeContext } from '../ts/ipc';
  import { toastStore } from '../ui';
  import {
    BrainCircuit,
    Check,
    Copy,
    RefreshCw,
    Terminal,
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

  // Load on mount; default selector to active namespace
  $effect(() => {
    if (!selectedNs) {
      selectedNs = namespace ?? '';
    }
  });

  // ─── Helpers ───────────────────────────────────────────────────────
  function tokenEstimate(text: string | null): number {
    if (!text) return 0;
    // Rough approximation: ~4 chars per token for English prose
    return Math.ceil(text.length / 4);
  }

  function memoryCount(text: string | null): number | null {
    // First line pattern: "Project memory: N memories in namespace 'x'"
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
  <div class="section-head">
    <div class="section-title-row">
      <BrainCircuit size={18} strokeWidth={1.75} />
      <h2 class="section-title">Context Preview</h2>
      <span class="preview-hint">exactly what your agent's LLM sees</span>
    </div>
    <div class="section-actions">
      <select
        class="ns-select"
        bind:value={selectedNs}
        disabled={loading}
        aria-label="Namespace"
        onchange={() => loadContext()}
      >
        <option value="">default</option>
        {#each namespaces as ns}
          <option value={ns}>{ns}</option>
        {/each}
      </select>
      <button class="btn btn-secondary btn-sm" onclick={loadContext} disabled={loading}>
        <span class:spinning={loading}><RefreshCw size={14} /></span>
        <span>Refresh</span>
      </button>
      <button
        class="btn btn-secondary btn-sm"
        onclick={copyContext}
        disabled={!contextText || loading}
        title="Copy context to clipboard"
      >
        {#if copied}
          <Check size={14} />
          <span>Copied</span>
        {:else}
          <Copy size={14} />
          <span>Copy</span>
        {/if}
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-banner">
      <Terminal size={16} />
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" onclick={loadContext} disabled={loading}>Retry</button>
    </div>
  {:else if loading && !contextText}
    <div class="loading-state"><span class="spinning"><RefreshCw size={16} /></span> Building context…</div>
  {:else if contextText}
    <!-- ─── Summary strip ───────────────────────────────────────── -->
    <div class="summary-strip">
      <div class="summary-item">
        <span class="summary-value">~{tokenEstimate(contextText)}</span>
        <span class="summary-label">tokens est.</span>
      </div>
      <div class="summary-item">
        <span class="summary-value">{memoryCount(contextText) ?? '—'}</span>
        <span class="summary-label">memories</span>
      </div>
      <div class="summary-item">
        <span class="summary-value">{lineCount(contextText)}</span>
        <span class="summary-label">lines</span>
      </div>
      <div class="summary-item summary-ns">
        <span class="summary-value">{selectedNs || 'default'}</span>
        <span class="summary-label">namespace</span>
      </div>
    </div>

    <!-- ─── The exact context string ────────────────────────────── -->
    <pre class="context-block">{contextText}</pre>
  {:else}
    <div class="empty-state">
      <BrainCircuit size={26} strokeWidth={1.5} />
      <p>No context loaded.</p>
      <button class="btn btn-sm btn-primary" onclick={loadContext} disabled={loading}>
        Load Context
      </button>
    </div>
  {/if}
</section>

<style>
  .context-section {
    margin-top: 1.5rem;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    margin-bottom: 0.75rem;
  }

  .section-title-row {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    color: var(--text-primary);
  }

  .section-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .preview-hint {
    font-size: 0.72rem;
    color: var(--text-muted);
    font-style: italic;
  }

  .section-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .ns-select {
    font-size: 0.78rem;
    padding: 0.35rem 0.5rem;
    border-radius: 6px;
    border: 1px solid var(--border-color, #ddd);
    background: var(--bg-secondary, #f5f5f5);
    color: var(--text-primary);
    max-width: 160px;
  }

  /* ─── Summary strip ──────────────────────────────────────────── */
  .summary-strip {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.6rem;
    margin-bottom: 0.75rem;
  }

  .summary-item {
    background: var(--bg-secondary, #f5f5f5);
    border-radius: 8px;
    padding: 0.55rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }

  .summary-value {
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .summary-label {
    font-size: 0.68rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .summary-ns .summary-value {
    font-size: 0.85rem;
    font-family: ui-monospace, monospace;
  }

  /* ─── Context block ──────────────────────────────────────────── */
  .context-block {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.76rem;
    line-height: 1.55;
    color: var(--text-primary);
    background: var(--bg-code, #f7f7f8);
    border: 1px solid var(--border-color, #e2e2e4);
    border-radius: 10px;
    padding: 1rem 1.15rem;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 340px;
    overflow-y: auto;
  }

  /* ─── States ─────────────────────────────────────────────────── */
  .loading-state {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 1.25rem;
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 1.5rem 1rem;
    color: var(--text-muted);
    font-size: 0.85rem;
    border: 1px dashed var(--border-color, #ddd);
    border-radius: 10px;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    background: color-mix(in srgb, #d93f0b 8%, transparent);
    border: 1px solid color-mix(in srgb, #d93f0b 25%, transparent);
    color: #d93f0b;
    font-size: 0.82rem;
  }

  .error-banner .btn {
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
