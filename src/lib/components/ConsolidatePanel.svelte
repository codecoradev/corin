<script lang="ts">
  import { consolidateMemories } from '../ts/ipc';
  import { Spinner, toastStore } from '../ui';
  import {
    Copy,
    GitMerge,
    RefreshCw,
    Search,
    CheckCircle2,
    AlertTriangle,
  } from 'lucide-svelte';

  interface Props {
    namespace: string | null;
  }

  let { namespace }: Props = $props();

  // ─── State ─────────────────────────────────────────────────────────
  let threshold = $state(0.85);
  let pairs = $state<SimilarPairUI[] | null>(null);
  let scanning = $state(false);
  let merging = $state(false);
  let skipped = $state<Set<number>>(new Set());
  let mergeResult = $state<ConsolidationResultUI | null>(null);
  let showConfirmMerge = $state(false);
  let error = $state<string | null>(null);

  interface SimilarPairUI {
    id_a: string;
    content_a: string;
    id_b: string;
    content_b: string;
    similarity: number;
  }

  interface ConsolidationResultUI {
    duplicates_found: number;
    merged: number;
    removed_ids: string[];
    kept_ids: string[];
  }

  // ─── Actions ───────────────────────────────────────────────────────
  async function findDuplicates() {
    scanning = true;
    error = null;
    pairs = null;
    mergeResult = null;
    skipped = new Set();
    try {
      const raw = (await consolidateMemories({
        threshold,
        dryRun: true,
        namespace: namespace ?? undefined,
      })) as SimilarPairUI[];
      pairs = Array.isArray(raw) ? raw : [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      scanning = false;
    }
  }

  async function mergeAll() {
    showConfirmMerge = false;
    merging = true;
    try {
      const raw = (await consolidateMemories({
        threshold,
        dryRun: false,
        namespace: namespace ?? undefined,
      })) as ConsolidationResultUI;
      mergeResult = raw;
      toastStore.success(`Merged ${raw.merged} duplicate pair${raw.merged === 1 ? '' : 's'}`);
      pairs = null;
      skipped = new Set();
    } catch (e) {
      toastStore.error(`Merge failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      merging = false;
    }
  }

  function toggleSkip(index: number) {
    const next = new Set(skipped);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    skipped = next;
  }

  function pendingCount(): number {
    if (!pairs) return 0;
    return pairs.length - skipped.size;
  }

  function resetScan() {
    pairs = null;
    mergeResult = null;
    skipped = new Set();
    error = null;
  }

  // ─── Helpers ───────────────────────────────────────────────────────
  function similarityPercent(sim: number): string {
    return `${(sim * 100).toFixed(0)}%`;
  }

  function similarityClass(sim: number): string {
    if (sim >= 0.95) return 'sim-high';
    if (sim >= 0.9) return 'sim-mid';
    return 'sim-low';
  }
</script>

<div class="consolidate-panel">
  <div class="section-header">
    <GitMerge size={18} strokeWidth={1.75} />
    <h2>Duplicate Detector</h2>
    {#if pairs}
      <span class="badge">{pendingCount()} pending</span>
    {/if}
  </div>
  <p class="section-desc">
    Find near-duplicate memories via cosine similarity. Preview first — review every pair before
    merging.
  </p>

  {#if error}
    <div class="error-banner">
      <AlertTriangle size={18} />
      <span>{error}</span>
    </div>
  {/if}

  {#if mergeResult}
    <div class="merge-result-banner">
      <CheckCircle2 size={18} />
      <span>
        Merge complete: <strong>{mergeResult.merged}</strong> merged,
        <strong>{mergeResult.removed_ids.length}</strong> duplicates removed
      </span>
      <button class="btn btn-sm btn-secondary" onclick={resetScan}>
        <RefreshCw size={13} />
        <span>New Scan</span>
      </button>
    </div>
  {/if}

  <!-- ─── Controls ─────────────────────────────────────────────── -->
  <div class="controls">
    <div class="threshold-control">
      <label for="threshold-slider">
        Similarity threshold: <strong>{threshold.toFixed(2)}</strong>
      </label>
      <input
        id="threshold-slider"
        type="range"
        min="0.5"
        max="0.95"
        step="0.05"
        bind:value={threshold}
        disabled={scanning || merging}
      />
      <div class="threshold-scale">
        <span>0.50 (loose)</span>
        <span>0.95 (strict)</span>
      </div>
    </div>
    {#if !pairs}
      <button
        class="btn btn-primary"
        onclick={findDuplicates}
        disabled={scanning || merging}
      >
        {#if scanning}
          <span class="spinning"><RefreshCw size={15} /></span>
          <span>Scanning…</span>
        {:else}
          <Search size={15} />
          <span>Find Duplicates</span>
        {/if}
      </button>
    {:else}
      <div class="scan-actions">
        <button class="btn btn-secondary" onclick={resetScan} disabled={merging}>
          <RefreshCw size={15} />
          <span>Rescan</span>
        </button>
        <button
          class="btn btn-primary"
          onclick={() => (showConfirmMerge = true)}
          disabled={merging || pendingCount() === 0}
          title={pendingCount() === 0 ? 'All pairs skipped' : ''}
        >
          {#if merging}
            <span class="spinning"><RefreshCw size={15} /></span>
            <span>Merging…</span>
          {:else}
            <GitMerge size={15} />
            <span>Merge All ({pendingCount()})</span>
          {/if}
        </button>
      </div>
    {/if}
  </div>

  {#if scanning}
    <div class="loading-state">
      <Spinner size={28} />
      <p>Scanning for duplicates at threshold {threshold.toFixed(2)}…</p>
    </div>
  {:else if pairs}
    {#if pairs.length === 0}
      <div class="empty-state">
        <CheckCircle2 size={28} strokeWidth={1.5} />
        <p>No duplicates found at this threshold.</p>
      </div>
    {:else}
      <div class="summary-line">
        <Copy size={14} />
        <span>
          {pairs.length} pair{pairs.length === 1 ? '' : 's'} found,
          {skipped.size} skipped
        </span>
      </div>
      <div class="pairs-list">
        {#each pairs as pair, i (pair.id_a + pair.id_b)}
          <div class="pair-card" class:skipped={skipped.has(i)}>
            <div class="pair-header">
              <span class="similarity-badge {similarityClass(pair.similarity)}">
                {similarityPercent(pair.similarity)} cosine
              </span>
              <div class="pair-actions">
                <button
                  class="btn btn-sm btn-ghost"
                  onclick={() => toggleSkip(i)}
                  disabled={merging}
                  title={skipped.has(i) ? 'Un-skip this pair' : 'Skip this pair'}
                >
                  {skipped.has(i) ? 'Unskip' : 'Skip'}
                </button>
              </div>
            </div>
            <div class="pair-contents">
              <div class="pair-side">
                <span class="side-label">Older</span>
                <p class="side-text">{pair.content_a}</p>
                <code class="side-id">{pair.id_a.slice(0, 12)}…</code>
              </div>
              <div class="pair-divider">
                <GitMerge size={14} strokeWidth={2} />
              </div>
              <div class="pair-side">
                <span class="side-label">Newer</span>
                <p class="side-text">{pair.content_b}</p>
                <code class="side-id">{pair.id_b.slice(0, 12)}…</code>
              </div>
            </div>
          </div>
        {/each}
      </div>
      <p class="merge-note">
        <AlertTriangle size={14} />
        Merging keeps the older memory's identity, merges content, and removes the newer duplicate.
        Server-side merge applies to all non-skipped pairs — review carefully.
      </p>
    {/if}
  {/if}

  <!-- ─── Confirm Merge Modal ───────────────────────────────────── -->
  {#if showConfirmMerge}
    <div
      class="modal-overlay"
      role="presentation"
      onclick={() => (showConfirmMerge = false)}
      onkeydown={(e) => e.key === 'Escape' && (showConfirmMerge = false)}
    >
      <div
        class="modal"
        role="dialog"
        tabindex={0}
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.key === 'Escape' && (showConfirmMerge = false)}
      >
        <div class="modal-icon">
          <GitMerge size={28} strokeWidth={1.5} />
        </div>
        <h3>Merge {pendingCount()} pair{pendingCount() === 1 ? '' : 's'}?</h3>
        <p class="modal-desc">
          This will merge all non-skipped duplicate pairs at threshold
          {threshold.toFixed(2)}. The older memory is kept and enriched; newer duplicates are
          permanently removed. This cannot be undone.
        </p>
        {#if skipped.size > 0}
          <p class="modal-warn">
            <strong>{skipped.size}</strong> pair{skipped.size === 1 ? '' : 's'} will be skipped.
          </p>
        {/if}
        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => (showConfirmMerge = false)}>
            Cancel
          </button>
          <button class="btn btn-primary" onclick={mergeAll} disabled={merging}>
            <GitMerge size={15} />
            Merge {pendingCount()}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .consolidate-panel {
    margin-top: 2rem;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.35rem;
  }

  .section-header h2 {
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .section-desc {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin: 0 0 1rem 0;
  }

  .badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-secondary, #eee);
    border-radius: 999px;
    padding: 0.15rem 0.6rem;
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
    font-size: 0.85rem;
    margin-bottom: 1rem;
  }

  .merge-result-banner {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    background: color-mix(in srgb, #0d7a4f 8%, transparent);
    border: 1px solid color-mix(in srgb, #0d7a4f 25%, transparent);
    color: #0d7a4f;
    font-size: 0.85rem;
    margin-bottom: 1rem;
  }

  .merge-result-banner .btn {
    margin-left: auto;
  }

  /* ─── Controls ────────────────────────────────────────────────── */
  .controls {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1.5rem;
    flex-wrap: wrap;
    padding: 1rem;
    border-radius: 10px;
    background: var(--bg-secondary, #f5f5f5);
    margin-bottom: 1.25rem;
  }

  .threshold-control {
    flex: 1;
    min-width: 240px;
  }

  .threshold-control label {
    font-size: 0.8rem;
    color: var(--text-secondary, #555);
    display: block;
    margin-bottom: 0.4rem;
  }

  .threshold-control input[type='range'] {
    width: 100%;
    accent-color: var(--accent, #0d7377);
  }

  .threshold-scale {
    display: flex;
    justify-content: space-between;
    font-size: 0.68rem;
    color: var(--text-muted, #999);
    margin-top: 0.2rem;
  }

  .scan-actions {
    display: flex;
    gap: 0.5rem;
  }

  /* ─── States ──────────────────────────────────────────────────── */
  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    padding: 2rem 1rem;
    color: var(--text-muted, #999);
    font-size: 0.85rem;
  }

  .empty-state {
    border: 1px dashed var(--border-color, #ddd);
    border-radius: 10px;
  }

  .summary-line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-bottom: 0.75rem;
  }

  /* ─── Pair cards ──────────────────────────────────────────────── */
  .pairs-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .pair-card {
    border: 1px solid var(--border-color, #ddd);
    border-radius: 10px;
    padding: 0.85rem 1rem;
    transition: opacity 0.15s ease;
  }

  .pair-card.skipped {
    opacity: 0.45;
  }

  .pair-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.7rem;
  }

  .similarity-badge {
    font-size: 0.72rem;
    font-weight: 600;
    padding: 0.18rem 0.6rem;
    border-radius: 999px;
  }

  .sim-high {
    background: color-mix(in srgb, #0d7a4f 12%, transparent);
    color: #0d7a4f;
  }

  .sim-mid {
    background: color-mix(in srgb, #b8860b 14%, transparent);
    color: #8a6508;
  }

  .sim-low {
    background: color-mix(in srgb, #555 12%, transparent);
    color: var(--text-secondary, #555);
  }

  .pair-contents {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    gap: 0.75rem;
    align-items: stretch;
  }

  .pair-side {
    background: var(--bg-secondary, #fafafa);
    border-radius: 8px;
    padding: 0.6rem 0.75rem;
    min-width: 0;
  }

  .side-label {
    display: block;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted, #999);
    margin-bottom: 0.3rem;
  }

  .side-text {
    font-size: 0.8rem;
    color: var(--text-primary, #222);
    margin: 0 0 0.35rem 0;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }

  .side-id {
    font-size: 0.65rem;
    color: var(--text-muted, #aaa);
  }

  .pair-divider {
    display: flex;
    align-items: center;
    color: var(--text-muted, #bbb);
  }

  .merge-note {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-top: 0.9rem;
  }

  /* ─── Modal ───────────────────────────────────────────────────── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-primary, #fff);
    border-radius: 12px;
    padding: 1.75rem;
    max-width: 420px;
    width: calc(100% - 2rem);
    text-align: center;
  }

  .modal-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent, #0d7377) 10%, transparent);
    color: var(--accent, #0d7377);
    margin: 0 auto 1rem auto;
  }

  .modal h3 {
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .modal-desc {
    font-size: 0.82rem;
    color: var(--text-secondary, #666);
    line-height: 1.5;
    margin: 0 0 1rem 0;
  }

  .modal-warn {
    font-size: 0.78rem;
    color: #8a6508;
    background: color-mix(in srgb, #b8860b 10%, transparent);
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    margin: 0 0 1rem 0;
  }

  .modal-actions {
    display: flex;
    gap: 0.6rem;
    justify-content: center;
  }

  .modal-actions .btn {
    min-width: 110px;
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
