<script lang="ts">
  import { Button, EmptyState, Spinner } from '../ui';
  import {
    fetchAlerts, filterAlerts, alertTrend, severityBreakdown,
    type ScanAlert, type Severity, type AlertFilter,
  } from '../utils/reviews';
  import { ShieldAlert, TrendingUp, RefreshCw } from 'lucide-svelte';

  const LS_REPO = 'corin_gh_repo';
  const LS_TOKEN = 'corin_gh_token';

  let repo = $state(localStorage.getItem(LS_REPO) ?? 'codecoradev/corin');
  let token = $state(localStorage.getItem(LS_TOKEN) ?? '');
  let loading = $state(false);
  let errorMsg = $state<string | null>(null);
  let alerts = $state<ScanAlert[]>([]);
  let loaded = $state(false);

  let filter = $state<AlertFilter>({ severity: 'all', state: 'open', query: '' });

  const breakdown = $derived(severityBreakdown(alerts));
  const trend = $derived(alertTrend(alerts, 12));
  const trendMax = $derived(Math.max(1, ...trend.map((t) => t.count)));
  const visible = $derived(filterAlerts(alerts, filter));

  async function load() {
    loading = true;
    errorMsg = null;
    try {
      localStorage.setItem(LS_REPO, repo);
      if (token) localStorage.setItem(LS_TOKEN, token);
      alerts = await fetchAlerts(repo, token, { state: 'all' });
      loaded = true;
    } catch (e: any) {
      errorMsg = e?.message ?? String(e);
      loaded = false;
    } finally {
      loading = false;
    }
  }

  function toggleSeverity(s: Severity) {
    filter = { ...filter, severity: filter.severity === s ? 'all' : s };
  }

  function sevClass(s: Severity): string {
    return s === 'critical' || s === 'high' ? 'sev-high' : s === 'medium' ? 'sev-med' : 'sev-low';
  }

  function fmtDate(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? '' : d.toLocaleDateString();
  }
</script>

<div class="review-history">
  <p class="hint">
    Lists GitHub Code Scanning alerts (Cora reviews upload SARIF via <code>cora upload-sarif</code>).
    Token needs the <code>security_events</code> scope — it is stored locally on this device only.
  </p>

  <div class="config-row">
    <input class="repo-input" bind:value={repo} placeholder="owner/repo" aria-label="Repository" />
    <input
      class="token-input"
      type="password"
      bind:value={token}
      placeholder="GitHub token (security_events)"
      aria-label="GitHub token"
    />
    <Button variant="primary" size="sm" onclick={load} disabled={loading || !repo.trim()}>
      {#if loading}<span class="spinning"><RefreshCw size={13} /></span>{:else}<ShieldAlert size={13} />{/if}
      Load
    </Button>
  </div>

  {#if errorMsg}
    <div class="error-msg">{errorMsg}</div>
  {/if}

  {#if loading}
    <div class="loading"><Spinner size={18} /> Loading alerts…</div>
  {:else if loaded && alerts.length === 0}
    <EmptyState
      icon={ShieldAlert}
      title="No code scanning alerts."
      subtitle="Either this repo is clean, or Cora reviews haven't been uploaded yet (cora upload-sarif)."
    />
  {:else if loaded}
    <div class="trend-row">
      <span class="trend-title"><TrendingUp size={13} /> new alerts / week</span>
      {#each trend as t (t.label)}
        <span class="trend-bar" style="height: {Math.max(3, (t.count / trendMax) * 26)}px;" title="{t.label}: {t.count}"></span>
      {/each}
    </div>

    <div class="breakdown">
      <button class="chip sev-high" class:off={filter.severity !== 'all' && filter.severity !== 'critical'} onclick={() => toggleSeverity('critical')}>
        {breakdown.critical} critical
      </button>
      <button class="chip sev-high" class:off={filter.severity !== 'all' && filter.severity !== 'high'} onclick={() => toggleSeverity('high')}>
        {breakdown.high} high
      </button>
      <button class="chip sev-med" class:off={filter.severity !== 'all' && filter.severity !== 'medium'} onclick={() => toggleSeverity('medium')}>
        {breakdown.medium} medium
      </button>
      <button class="chip sev-low" class:off={filter.severity !== 'all' && filter.severity !== 'low'} onclick={() => toggleSeverity('low')}>
        {breakdown.low} low
      </button>
      <span class="chip muted">{breakdown.none} unranked</span>
    </div>

    <div class="filter-row">
      <select bind:value={filter.state} aria-label="State filter">
        <option value="open">Open</option>
        <option value="fixed">Fixed</option>
        <option value="dismissed">Dismissed</option>
        <option value="all">All states</option>
      </select>
      <input bind:value={filter.query} placeholder="Filter by rule, file, or message…" aria-label="Text filter" />
      <span class="count">{visible.length} shown</span>
    </div>

    <div class="alert-list">
      {#each visible as a (a.number)}
        <div class="alert-row">
          <span class="sev-pill {sevClass(a.severity)}">{a.severity}</span>
          <div class="alert-main">
            <div class="alert-top">
              <span class="rule">{a.rule}</span>
              <span class="state state-{a.state}">{a.state}</span>
            </div>
            <p class="msg">{a.message || a.description}</p>
            <div class="meta">
              {#if a.file}<code>{a.file}{a.line ? `:${a.line}` : ''}</code>{/if}
              <span>{fmtDate(a.created_at)}</span>
              <span>#{a.number}</span>
            </div>
          </div>
        </div>
      {:else}
        <p class="none">No alerts match the current filters.</p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0 0 10px;
    line-height: 1.5;
  }
  .hint code {
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .config-row {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .config-row input {
    padding: 7px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.82rem;
  }
  .repo-input { flex: 0 0 220px; }
  .token-input { flex: 1; min-width: 180px; }

  .error-msg {
    padding: 8px 12px;
    background: var(--color-red-bg);
    color: var(--red);
    border: 1px solid var(--color-red-line);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    margin-bottom: 12px;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    padding: 16px 0;
    font-size: 0.85rem;
  }

  .trend-row {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    margin-bottom: 14px;
  }
  .trend-title {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.72rem;
    color: var(--text-muted);
    margin-right: 6px;
  }
  .trend-bar {
    width: 10px;
    background: var(--yellow);
    border-radius: 2px 2px 0 0;
    opacity: 0.85;
  }

  .breakdown {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .chip {
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .chip.muted { cursor: default; opacity: 0.6; }
  .chip.off { opacity: 0.45; }
  .chip.sev-high { border-color: var(--color-red-line); color: var(--red); }
  .chip.sev-med { border-color: rgba(249, 226, 175, 0.4); color: var(--yellow); }
  .chip.sev-low { border-color: rgba(166, 227, 161, 0.4); color: var(--green); }

  .filter-row {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 10px;
  }
  .filter-row select,
  .filter-row input {
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.8rem;
  }
  .filter-row input { flex: 1; }
  .filter-row .count {
    font-size: 0.72rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .alert-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 420px;
    overflow-y: auto;
  }
  .alert-row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .sev-pill {
    flex-shrink: 0;
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 2px 7px;
    border-radius: var(--radius-pill);
    margin-top: 2px;
  }
  .sev-pill.sev-high { background: var(--color-red-bg); color: var(--red); }
  .sev-pill.sev-med { background: rgba(249, 226, 175, 0.12); color: var(--yellow); }
  .sev-pill.sev-low { background: rgba(166, 227, 161, 0.12); color: var(--green); }
  .sev-pill.sev-none { background: var(--bg-hover); color: var(--text-muted); }

  .alert-main { flex: 1; min-width: 0; }
  .alert-top {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .rule {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .state {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-muted);
  }
  .state-open { color: var(--yellow); }
  .state-fixed { color: var(--green); }
  .msg {
    margin: 2px 0 4px;
    font-size: 0.78rem;
    color: var(--text-secondary);
    text-wrap: pretty;
  }
  .meta {
    display: flex;
    gap: 10px;
    font-size: 0.68rem;
    color: var(--text-muted);
  }
  .meta code {
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .none {
    color: var(--text-muted);
    font-size: 0.8rem;
    text-align: center;
    padding: 12px 0;
  }

  .spinning {
    display: inline-flex;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
