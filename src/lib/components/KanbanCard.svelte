<script lang="ts">
  import type { KanbanTask } from '../ts/types';

  interface Props {
    task: KanbanTask;
    onclick: (task: KanbanTask) => void;
  }

  let { task, onclick }: Props = $props();

  // Status color mapping
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

  // Format age for display
  function formatAge(seconds: number | null): string {
    if (!seconds || seconds < 0) return '';
    const hours = Math.floor(seconds / 3600);
    if (hours < 1) return '< 1h';
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    return `${days}d`;
  }

  let statusColor = $derived(statusColors[task.status] || 'var(--text-muted)');
  let startedAge = $derived(formatAge(task.age?.started_age_seconds ?? null));
  let createdAge = $derived(formatAge(task.age?.created_age_seconds ?? null));
  let hasDiagnostics = $derived(
    task.warnings && (task.warnings as { count: number }).count > 0
  );
</script>

<button class="kanban-card" onclick={() => onclick(task)}>
  <div class="card-header">
    <span class="card-title">{task.title}</span>
    {#if task.link_counts && (task.link_counts.parents > 0 || task.link_counts.children > 0)}
      <span class="link-badge" title="Parents: {task.link_counts.parents}, Children: {task.link_counts.children}">
        {task.link_counts.parents > 0 ? '⬆' : ''}
        {task.link_counts.children > 0 ? '⬇' : ''}
      </span>
    {/if}
  </div>

  <div class="card-meta">
    {#if task.assignee}
      <span class="assignee" title={task.assignee}>{task.assignee}</span>
    {/if}
    {#if task.comment_count && task.comment_count > 0}
      <span class="comment-badge" title="{task.comment_count} comments">
        💬 {task.comment_count}
      </span>
    {/if}
    {#if hasDiagnostics}
      <span class="warn-badge" title="Diagnostics warning">⚠</span>
    {/if}
    {#if task.progress}
      <span class="progress-badge" title="Sub-tasks: {task.progress.done}/{task.progress.total}">
        {task.progress.done}/{task.progress.total}
      </span>
    {/if}
  </div>

  <div class="card-footer">
    <span class="status-dot" style="background: {statusColor}" title={task.status}></span>
    {#if startedAge}
      <span class="age" title="Time since started">🕐 {startedAge}</span>
    {:else if createdAge}
      <span class="age" title="Time since created">🕐 {createdAge}</span>
    {/if}
    <span class="priority" title="Priority: {task.priority}">
      P{task.priority}
    </span>
  </div>
</button>

<style>
  .kanban-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
    text-align: left;
    width: 100%;
    font-family: var(--font-sans);
  }

  .kanban-card:hover {
    border-color: var(--accent-dim);
    background: var(--bg-hover);
  }

  .card-header {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }

  .card-title {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.3;
    flex: 1;
    word-break: break-word;
  }

  .link-badge {
    font-size: 0.7rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .assignee {
    font-size: 0.75rem;
    color: var(--accent);
    background: var(--accent-dim);
    padding: 1px 6px;
    border-radius: 3px;
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .comment-badge,
  .warn-badge,
  .progress-badge {
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .card-footer {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .age {
    font-size: 0.7rem;
    color: var(--text-muted);
    flex: 1;
  }

  .priority {
    font-size: 0.65rem;
    color: var(--text-muted);
    background: var(--bg-secondary);
    padding: 0 4px;
    border-radius: 3px;
    font-family: var(--font-mono);
  }
</style>
