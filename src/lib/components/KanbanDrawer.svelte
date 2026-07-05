<script lang="ts">
  import type { KanbanTask, KanbanTaskDetail, KanbanComment, KanbanEvent, KanbanRun } from '../ts/types';
  import { kanban } from '../ts/ipc';
  import KanbanCard from './KanbanCard.svelte';

  interface Props {
    taskId: string | null;
    onclose: () => void;
    onrefresh: () => void;
  }

  let { taskId, onclose, onrefresh }: Props = $props();

  let detail = $state<KanbanTaskDetail | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let commentText = $state('');
  let submitting = $state(false);

  // Status color
  const statusColors: Record<string, string> = {
    triage: 'var(--peach)',
    todo: 'var(--text-muted)',
    scheduled: 'var(--mauve)',
    ready: 'var(--accent)',
    running: 'var(--teal)',
    blocked: 'var(--red)',
    review: 'var(--yellow)',
    done: 'var(--green)',
  };

  let statusColor = $derived(detail ? (statusColors[detail.task.status] || 'var(--text-muted)') : 'var(--text-muted)');

  // Fetch task detail when taskId changes
  $effect(() => {
    if (!taskId) {
      detail = null;
      return;
    }
    fetchDetail(taskId);
  });

  async function fetchDetail(id: string) {
    loading = true;
    error = null;
    try {
      detail = await kanban.task(id);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function submitComment() {
    if (!taskId || !commentText.trim()) return;
    submitting = true;
    try {
      await kanban.addComment(taskId, commentText.trim());
      commentText = '';
      await fetchDetail(taskId);
      onrefresh();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function formatTimestamp(ts: number | null): string {
    if (!ts) return '—';
    const d = new Date(ts * 1000);
    const now = Date.now();
    const diff = now - d.getTime();
    if (diff < 60_000) return 'just now';
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }

  function formatDuration(start: number | null, end: number | null): string {
    if (!start) return '—';
    const s = ((end || Math.floor(Date.now() / 1000)) - start);
    if (s < 60) return `${s}s`;
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    return `${(s / 3600).toFixed(1)}h`;
  }

  // Event kind icons
  function eventIcon(kind: string): string {
    const icons: Record<string, string> = {
      created: '📋',
      claimed: '🚀',
      spawned: '⚡',
      completed: '✅',
      failed: '❌',
      reclaimed: '🔄',
      blocked: '🚫',
      edited: '✏️',
      reprioritized: '📊',
      heartbeat: '💓',
      comment: '💬',
      default: '•',
    };
    return icons[kind] || icons.default;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }

  $effect(() => {
    if (taskId) window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if taskId}
  <div class="drawer-overlay" onclick={onclose}>
    <div class="drawer" onclick={(e) => e.stopPropagation()}>
      <div class="drawer-header">
        <span class="drawer-status-dot" style="background: {statusColor}"></span>
        <span class="drawer-status">{detail?.task.status || ''}</span>
        <button class="drawer-close" onclick={onclose}>✕</button>
      </div>

      {#if loading}
        <div class="drawer-loading">Loading...</div>
      {:else if error}
        <div class="drawer-error">{error}</div>
      {:else if detail}
        <div class="drawer-body">
          <!-- Task info -->
          <div class="drawer-section">
            <h3 class="drawer-title">{detail.task.title}</h3>
            <div class="drawer-meta">
              {#if detail.task.assignee}
                <span class="meta-tag assignee-tag">@{detail.task.assignee}</span>
              {/if}
              <span class="meta-tag" title="Priority">P{detail.task.priority}</span>
              {#if detail.task.tenant}
                <span class="meta-tag" title="Tenant">{detail.task.tenant}</span>
              {/if}
              <span class="meta-tag" title="Created">{formatTimestamp(detail.task.created_at)}</span>
            </div>
          </div>

          <!-- Summary -->
          {#if detail.task.latest_summary}
            <div class="drawer-section">
              <h4>Summary</h4>
              <p class="summary-text">{detail.task.latest_summary}</p>
            </div>
          {/if}

          <!-- Body -->
          {#if detail.task.body}
            <div class="drawer-section">
              <h4>Description</h4>
              <pre class="body-text">{detail.task.body}</pre>
            </div>
          {/if}

          <!-- Links -->
          {#if detail.links.parents.length > 0 || detail.links.children.length > 0}
            <div class="drawer-section">
              <h4>Links</h4>
              <div class="links-list">
                {#each detail.links.parents as pid}
                  <span class="link-tag parent" onclick={() => { taskId = pid; }}>⬆ {pid.slice(0, 12)}</span>
                {/each}
                {#each detail.links.children as cid}
                  <span class="link-tag child" onclick={() => { taskId = cid; }}>⬇ {cid.slice(0, 12)}</span>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Run history -->
          {#if detail.runs.length > 0}
            <div class="drawer-section">
              <h4>Run History ({detail.runs.length})</h4>
              <div class="runs-list">
                {#each detail.runs as run (run.id)}
                  <div class="run-entry" class:run-active={run.status === 'running'}>
                    <div class="run-header">
                      <span class="run-profile">{run.profile}</span>
                      <span class="run-status">{run.status}</span>
                    </div>
                    {#if run.summary}
                      <p class="run-summary">{run.summary}</p>
                    {/if}
                    {#if run.error}
                      <p class="run-error">{run.error}</p>
                    {/if}
                    <div class="run-footer">
                      <span class="run-duration">
                        {formatDuration(run.started_at, run.ended_at)}
                      </span>
                      {#if run.outcome}
                        <span class="run-outcome">{run.outcome}</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Comments -->
          <div class="drawer-section">
            <h4>Comments ({detail.comments.length})</h4>
            {#each detail.comments as comment (comment.id)}
              <div class="comment">
                <div class="comment-header">
                  <span class="comment-author">{comment.author}</span>
                  <span class="comment-time">{formatTimestamp(comment.created_at)}</span>
                </div>
                <p class="comment-body">{comment.body}</p>
              </div>
            {/each}

            <div class="comment-form">
              <textarea
                class="comment-input"
                placeholder="Add a comment..."
                rows="2"
                bind:value={commentText}
              ></textarea>
              <button
                class="comment-submit"
                onclick={submitComment}
                disabled={!commentText.trim() || submitting}
              >
                {submitting ? 'Sending...' : 'Comment'}
              </button>
            </div>
          </div>

          <!-- Events -->
          <div class="drawer-section">
            <h4>Events ({detail.events.length})</h4>
            <div class="events-list">
              {#each detail.events.slice().reverse() as event (event.id)}
                <div class="event-entry">
                  <span class="event-icon">{eventIcon(event.kind)}</span>
                  <span class="event-kind">{event.kind}</span>
                  <span class="event-time">{formatTimestamp(event.created_at)}</span>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .drawer-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 100;
    display: flex;
    justify-content: flex-end;
  }

  .drawer {
    width: 480px;
    max-width: 90vw;
    height: 100vh;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    font-family: var(--font-sans);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .drawer-status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .drawer-status {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    flex: 1;
  }

  .drawer-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px;
    line-height: 1;
  }
  .drawer-close:hover { color: var(--text-primary); }

  .drawer-loading,
  .drawer-error {
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
  }
  .drawer-error { color: var(--red); }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .drawer-section {
    margin-bottom: 20px;
  }

  .drawer-section h3 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 8px;
    line-height: 1.4;
  }

  .drawer-section h4 {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 8px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .drawer-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .meta-tag {
    font-size: 0.7rem;
    color: var(--text-muted);
    background: var(--bg-primary);
    padding: 2px 8px;
    border-radius: 3px;
    font-family: var(--font-mono);
  }

  .assignee-tag {
    color: var(--accent);
    background: var(--accent-dim);
    font-family: var(--font-sans);
  }

  .summary-text {
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
    white-space: pre-wrap;
  }

  .body-text {
    font-size: 0.8rem;
    color: var(--text-muted);
    line-height: 1.5;
    margin: 0;
    white-space: pre-wrap;
    font-family: var(--font-mono);
    background: var(--bg-primary);
    padding: 8px 10px;
    border-radius: var(--radius);
    overflow-x: auto;
    max-height: 300px;
  }

  .links-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .link-tag {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    padding: 2px 8px;
    border-radius: 3px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .link-tag:hover { opacity: 0.8; }
  .link-tag.parent { color: var(--peach); background: rgba(250, 179, 135, 0.15); }
  .link-tag.child { color: var(--teal); background: rgba(148, 226, 213, 0.15); }

  .runs-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .run-entry {
    background: var(--bg-primary);
    border-radius: var(--radius);
    padding: 8px 10px;
    border-left: 3px solid var(--border);
  }
  .run-entry.run-active { border-left-color: var(--teal); }

  .run-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .run-profile { font-size: 0.8rem; color: var(--accent); font-weight: 500; }
  .run-status { font-size: 0.7rem; color: var(--text-muted); text-transform: uppercase; }

  .run-summary {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin: 4px 0 0;
    line-height: 1.4;
  }
  .run-error {
    font-size: 0.8rem;
    color: var(--red);
    margin: 4px 0 0;
    font-family: var(--font-mono);
  }

  .run-footer {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .comment {
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .comment-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 4px;
  }
  .comment-author { font-size: 0.8rem; color: var(--accent); font-weight: 500; }
  .comment-time { font-size: 0.7rem; color: var(--text-muted); }

  .comment-body {
    font-size: 0.8rem;
    color: var(--text-secondary);
    line-height: 1.4;
    margin: 0;
    white-space: pre-wrap;
  }

  .comment-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 12px;
  }

  .comment-input {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 0.85rem;
    padding: 8px 10px;
    resize: vertical;
    min-height: 40px;
  }
  .comment-input:focus { outline: none; border-color: var(--accent); }

  .comment-submit {
    align-self: flex-end;
    padding: 6px 16px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .comment-submit:disabled { opacity: 0.4; cursor: default; }
  .comment-submit:hover:not(:disabled) { opacity: 0.85; }

  .events-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 200px;
    overflow-y: auto;
  }

  .event-entry {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
    padding: 3px 0;
  }
  .event-icon { flex-shrink: 0; }
  .event-kind { color: var(--text-secondary); flex: 1; }
  .event-time { color: var(--text-muted); }
</style>
