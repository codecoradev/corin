<script lang="ts">
  import { agents as agentApi } from '../../ts/ipc';
  import { formatDuration, formatTime } from '../../utils/format';

  // Agent detection state
  let detectedAgents = $state<Array<{ name: string; config_path: string; found: boolean }>>([]);
  let generatingMd = $state(false);
  let agentMdPath = $state<string | null>(null);

  // Dream cycle state
  let runningDream = $state(false);
  let dreamResult = $state<{
    success: boolean;
    phases: Array<{ phase: string; status: string; summary: string; changes: number; warnings: number }>;
    total_changes: number;
    total_warnings: number;
    total_errors: number;
    dry_run: boolean;
    duration_ms: number;
    hint?: string;
  } | null>(null);
  let dreamHistory = $state<Array<{
    id: number;
    ran_at: string;
    success: boolean;
    total_changes: number;
    total_warnings: number;
    total_errors: number;
    duration_ms: number;
    phases: Array<{ phase: string; status: string; summary: string; changes: number; warnings: number }>;
  }>>([]);
  let dreamHistoryLoaded = $state(false);

  async function loadAgents() {
    try { detectedAgents = await agentApi.detect(); } catch { /* ignore */ }
  }

  async function genAgentMd() {
    generatingMd = true;
    try { agentMdPath = await agentApi.generateAgentMd(); } catch { /* ignore */ }
    generatingMd = false;
  }

  async function runDream() {
    runningDream = true;
    dreamResult = null;
    try {
      const resp = await agentApi.runDream();
      dreamResult = resp;
      if (resp.success) await loadDreamHistory();
    } catch {
      dreamResult = { success: false, phases: [], total_changes: 0, total_warnings: 0, total_errors: 1, dry_run: false, duration_ms: 0, hint: 'Failed to run dream cycle' };
    }
    runningDream = false;
  }

  async function loadDreamHistory() {
    try {
      dreamHistory = await agentApi.getDreamHistory(5);
      dreamHistoryLoaded = true;
    } catch { /* ignore */ }
  }

  // Load on mount
  $effect(() => {
    loadAgents();
    loadDreamHistory();
  });
</script>

<section class="content-section">
  <h3>Detected Agents</h3>
  <p class="setting-hint">AI coding agents that can use uteke memory.</p>
  <div class="agent-list">
    {#each detectedAgents as agent}
      <div class="agent-item" class:found={agent.found}>
        <span class="agent-status">{agent.found ? '✓' : '○'}</span>
        <div class="agent-info">
          <span class="agent-name">{agent.name}</span>
          <code class="agent-path">{agent.config_path}</code>
        </div>
      </div>
    {/each}
  </div>
</section>

<section class="content-section">
  <h3>Agent Instructions (.agent.md)</h3>
  <p class="setting-hint">Generate a <code>.agent.md</code> file with memory protocol instructions for AI agents.</p>
  <button class="data-btn" onclick={genAgentMd} disabled={generatingMd}>
    {generatingMd ? 'Generating...' : '✦ Generate .agent.md'}
  </button>
  {#if agentMdPath}
    <p class="setting-hint" style="margin-top:8px;">
      ✓ Written to <code>{agentMdPath}</code>
    </p>
  {/if}
</section>

<section class="content-section">
  <h3>Dream Cycle (Maintenance)</h3>
  <p class="setting-hint">Run uteke's maintenance pipeline: lint → backlinks → dedup → orphans → compact → verify.</p>
  <div class="dream-actions">
    <button class="dream-btn" onclick={runDream} disabled={runningDream}>
      {runningDream ? 'Running...' : '☾ Run Dream Cycle'}
    </button>
  </div>

  {#if dreamResult}
    <div class="dream-result">
      <div class="dream-summary" class:ok={dreamResult.success} class:err={!dreamResult.success}>
        <span class="dream-status">{dreamResult.success ? '✓' : '✗'}</span>
        <span>
          {dreamResult.total_changes} changes,
          {dreamResult.total_warnings} warnings,
          {dreamResult.total_errors} errors
        </span>
        <span class="dream-duration">{formatDuration(dreamResult.duration_ms)}</span>
      </div>
      {#if dreamResult.hint}
        <p class="dream-hint">{dreamResult.hint}</p>
      {/if}
      {#if dreamResult.phases.length > 0}
        <div class="dream-phases">
          {#each dreamResult.phases as phase}
            <div class="dream-phase" class:phase-ok={phase.status === 'ok'} class:phase-warn={phase.status === 'warning'} class:phase-err={phase.status === 'error'}>
              <span class="phase-name">{phase.phase}</span>
              <span class="phase-summary">{phase.summary}</span>
              {#if phase.changes > 0}
                <span class="phase-changes">+{phase.changes}</span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if dreamHistoryLoaded && dreamHistory.length > 0}
    <div class="dream-history">
      <h4>Recent Runs</h4>
      <div class="history-list">
        {#each dreamHistory as run}
          <div class="history-item" class:history-ok={run.success} class:history-err={!run.success}>
            <span class="history-status">{run.success ? '✓' : '✗'}</span>
            <span class="history-time">{formatTime(run.ran_at)}</span>
            <span class="history-stats">{run.total_changes}c {run.total_warnings}w {run.total_errors}e</span>
            <span class="history-duration">{formatDuration(run.duration_ms)}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  .agent-list { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
  .agent-item { display: flex; align-items: flex-start; gap: 10px; padding: 8px 12px; border-radius: 6px; background: rgba(255,255,255,0.03); }
  .agent-item.found { background: rgba(166,227,161,0.08); }
  .agent-status { font-size: 1.1rem; font-weight: 700; min-width: 20px; }
  .agent-item.found .agent-status { color: var(--green); }
  .agent-info { display: flex; flex-direction: column; gap: 2px; }
  .agent-name { font-weight: 600; font-size: 0.9rem; }
  .agent-path { font-size: 0.75rem; color: var(--text-muted); }
  .dream-result { margin-top: 8px; font-size: 0.85rem; }
  .dream-actions { margin-top: 8px; }
  .dream-btn {
    padding: 8px 20px;
    background: linear-gradient(135deg, rgba(137, 180, 250, 0.15), rgba(203, 166, 247, 0.15));
    color: var(--accent);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }
  .dream-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: linear-gradient(135deg, rgba(137, 180, 250, 0.25), rgba(203, 166, 247, 0.25));
  }
  .dream-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .dream-summary {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 6px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
  }
  .dream-summary.ok .dream-status { color: var(--green); }
  .dream-summary.err .dream-status { color: var(--red); }
  .dream-summary.err { border-color: rgba(243, 139, 168, 0.3); }
  .dream-status { font-size: 1.1rem; font-weight: 700; }
  .dream-duration { margin-left: auto; color: var(--text-muted); font-size: 0.8rem; }
  .dream-hint { margin-top: 6px; font-size: 0.8rem; color: var(--text-muted); }
  .dream-phases { margin-top: 8px; display: flex; flex-direction: column; gap: 4px; }
  .dream-phase {
    display: flex; align-items: center; gap: 10px;
    padding: 6px 10px; border-radius: 4px;
    background: rgba(255,255,255,0.02);
    font-size: 0.8rem;
  }
  .dream-phase.phase-warn { background: rgba(249, 226, 175, 0.08); }
  .dream-phase.phase-err { background: rgba(243, 139, 168, 0.08); }
  .phase-name {
    font-weight: 600; font-family: var(--font-mono);
    min-width: 70px; color: var(--accent);
  }
  .phase-summary { flex: 1; color: var(--text-secondary); }
  .phase-changes { color: var(--green); font-weight: 600; font-size: 0.75rem; }
  .dream-history { margin-top: 16px; }
  .dream-history h4 { font-size: 0.8rem; color: var(--text-muted); margin: 0 0 8px; font-weight: 600; }
  .history-list { display: flex; flex-direction: column; gap: 4px; }
  .history-item {
    display: flex; align-items: center; gap: 10px;
    padding: 5px 10px; border-radius: 4px;
    font-size: 0.78rem;
    background: rgba(255,255,255,0.02);
  }
  .history-item.history-ok .history-status { color: var(--green); }
  .history-item.history-err .history-status { color: var(--red); }
  .history-status { font-weight: 700; min-width: 16px; }
  .history-time { color: var(--text-secondary); }
  .history-stats { color: var(--text-muted); font-family: var(--font-mono); font-size: 0.72rem; }
  .history-duration { margin-left: auto; color: var(--text-muted); font-size: 0.72rem; }
  .data-btn {
    padding: 8px 16px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .data-btn:hover { border-color: var(--accent); }
  .setting-hint { font-size: 0.8rem; color: var(--text-muted); margin: 0 0 8px; }
  h3 { font-size: 0.9rem; color: var(--text-secondary); margin: 0 0 14px; font-weight: 600; }
  .content-section { margin-bottom: 24px; }
  .content-section:last-child { margin-bottom: 0; }
</style>
