<script lang="ts">
  import { memory as memoryApi, uteke, utekeServer, memoryDocRefs, memoryFeedback, memoryTimeline } from '../ts/ipc';
  import type { MemoryEntry, TimelineEvent } from '../ts/types';
  import { X, Link2, FileText, ThumbsUp, ThumbsDown, Clock } from 'lucide-svelte';
  import { ConfirmDialog, Spinner, toastStore } from '../ui';

  interface Neighbor {
    id: string;
    content: string;
    tags: string[];
    namespace: string | null;
    importance: number | null;
    content_type: string | null;
    created_at: string | null;
    relationship: string;
    score: number | null;
    shared_tags: string[];
  }

  interface Props {
    memoryId: string;
    onedit: (m: MemoryEntry) => void;
    onback: () => void;
    onneighborclick: (id: string) => void;
    ondeleted?: () => void;
  }

  let { memoryId, onedit, onback, onneighborclick, ondeleted }: Props = $props();

  let memory = $state<MemoryEntry | null>(null);
  let neighbors = $state<Neighbor[]>([]);
  let docSlugs = $state<string[]>([]);
  let loading = $state(true);
  let showDeleteConfirm = $state(false);

  // Trust feedback state (#207)
  let feedbackGiven = $state<'helpful' | 'unhelpful' | null>(null);
  let feedbackDelta = $state<number | null>(null);
  let submittingFeedback = $state(false);

  // Timeline state
  let timeline = $state<TimelineEvent[]>([]);
  let timelineLoading = $state(false);
  let timelineExpanded = $state(false);

  async function load() {
    loading = true;
    try {
      try {
        memory = await memoryApi.get(memoryId);
      } catch {
        memory = await uteke.get(memoryId);
      }
      // Load neighbors from Uteke (shared tags + explicit edges)
      neighbors = await uteke.neighbors(memoryId, 20).catch(() => []);
      // Cross-entity linking (#207): documents that reference this memory.
      // Non-fatal — older uteke-serve builds lack the endpoint.
      try {
        const refs = await memoryDocRefs(memoryId);
        docSlugs = refs.doc_slugs ?? [];
      } catch {
        docSlugs = [];
      }
      // Timeline events (created, updated, recalled, etc.)
      // Non-fatal — older uteke-serve builds lack the endpoint.
      try {
        timelineLoading = true;
        timeline = await memoryTimeline(memoryId, 50);
      } catch {
        timeline = [];
      } finally {
        timelineLoading = false;
      }
    } catch {
      memory = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    memoryId;
    // Reset feedback state when switching memories (#207 security fix).
    feedbackGiven = null;
    feedbackDelta = null;
    submittingFeedback = false;
    // Reset timeline state
    timeline = [];
    timelineExpanded = false;
    load();
  });

  async function handleDelete() {
    // Try server delete first (Uteke memory), fallback to Hub DB
    try {
      const status = await utekeServer.status();
      if (status.available) {
        await utekeServer.forget(memoryId);
      } else {
        await memoryApi.forget(memoryId);
      }
      // Surface success: refresh the underlying list + toast. Just closing
      // the panel is ambiguous (looks like nothing happened).
      if (ondeleted) {
        ondeleted();
      } else {
        onback();
      }
    } catch (e) {
      toastStore.error(`Failed to delete memory: ${e instanceof Error ? e.message : String(e)}`);
      showDeleteConfirm = false;
    }
  }

  // Relationship badge styling
  function relColor(rel: string): string {
    if (rel.startsWith('references')) return 'refs';
    if (rel.startsWith('supersedes')) return 'super';
    if (rel.startsWith('replies_to')) return 'reply';
    if (rel.startsWith('shared_tag')) return 'shared';
    if (rel.startsWith('similar')) return 'sim';
    return 'related';
  }

  // Click handler for doc slug links — navigation wiring comes later (#207 phase 2).
  function handleDocClick(_slug: string) {
    // Intentional no-op until document navigation is wired.
  }

  // Timeline event helpers
  const EVENT_META: Record<string, { color: string; label: string }> = {
    created:      { color: 'green',  label: 'Created' },
    updated:      { color: 'blue',   label: 'Updated' },
    recalled:     { color: 'mauve',  label: 'Recalled' },
    consolidated: { color: 'yellow', label: 'Consolidated' },
    tagged:       { color: 'peach',  label: 'Tagged' },
    forgot:       { color: 'red',    label: 'Forgotten' },
  };

  function eventColor(type: string): string {
    return EVENT_META[type]?.color ?? 'teal';
  }

  function eventLabel(type: string): string {
    return EVENT_META[type]?.label ?? type;
  }

  function timeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const sec = Math.floor(diff / 1000);
    if (sec < 60) return 'just now';
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min}m ago`;
    const hr = Math.floor(min / 60);
    if (hr < 24) return `${hr}h ago`;
    const day = Math.floor(hr / 24);
    if (day < 30) return `${day}d ago`;
    const mo = Math.floor(day / 30);
    if (mo < 12) return `${mo}mo ago`;
    return `${Math.floor(mo / 12)}y ago`;
  }

  // Trust feedback handler (#207)
  async function handleFeedback(type: 'helpful' | 'unhelpful') {
    if (submittingFeedback || feedbackGiven === type) return;
    submittingFeedback = true;
    try {
      const res = await memoryFeedback(memoryId, type);
      feedbackGiven = type;
      feedbackDelta = res.delta;
      if (type === 'helpful') {
        toastStore.success('Marked as helpful');
      } else {
        toastStore.info('Marked as unhelpful — importance reduced');
      }
    } catch (e) {
      toastStore.error(`Feedback failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      submittingFeedback = false;
    }
  }
</script>

<div class="memory-detail">
  <div class="detail-header">
    <button class="back-btn" onclick={onback}><X size={13} strokeWidth={2} /> Close <kbd>Esc</kbd></button>
    {#if memory}
      <div class="header-actions">
        <button class="edit-btn" onclick={() => onedit(memory!)}>Edit</button>
        <button class="delete-btn" onclick={() => (showDeleteConfirm = true)}>Delete</button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="loading"><Spinner size={18} /> Loading...</div>
  {:else if !memory}
    <div class="empty">Memory not found.</div>
  {:else}
    <div class="detail-body">
      <div class="content-section">
        <pre class="content-text">{memory.content}</pre>

        <div class="meta-grid">
          {#if memory.tags.length > 0}
            <div class="meta-row">
              <span class="meta-label">Tags</span>
              <div class="tags">
                {#each memory.tags as tag}<span class="tag">{tag}</span>{/each}
              </div>
            </div>
          {/if}
          {#if memory.namespace}
            <div class="meta-row">
              <span class="meta-label">Namespace</span>
              <span>{memory.namespace}</span>
            </div>
          {/if}
          {#if memory.content_type}
            <div class="meta-row">
              <span class="meta-label">Type</span>
              <span>{memory.content_type}</span>
            </div>
          {/if}
          {#if memory.importance !== null}
            <div class="meta-row">
              <span class="meta-label">Importance</span>
              <span>{(memory.importance * 100).toFixed(0)}%</span>
            </div>
          {/if}
          {#if memory.created_at}
            <div class="meta-row">
              <span class="meta-label">Created</span>
              <span>{new Date(memory.created_at).toLocaleString()}</span>
            </div>
          {/if}
        </div>
      </div>

      <div class="docs-section">
        <div class="docs-header">
          <h3><FileText size={14} strokeWidth={2} class="doc-icon" /> Referenced Documents ({docSlugs.length})</h3>
        </div>

        {#if docSlugs.length === 0}
          <div class="no-docs">
            <p>No linked documents.</p>
          </div>
        {:else}
          <div class="doc-list">
            {#each docSlugs as slug}
              <a
                href="#"
                class="doc-link"
                onclick={(e) => {
                  e.preventDefault();
                  handleDocClick(slug);
                }}
              >
                <FileText size={12} strokeWidth={2} />
                {slug}
              </a>
            {/each}
          </div>
        {/if}
      </div>

      <div class="feedback-section">
        <span class="feedback-label">Was this helpful?</span>
        <div class="feedback-buttons">
          <button
            class="feedback-btn up {feedbackGiven === 'helpful' ? 'active' : ''}"
            disabled={submittingFeedback || feedbackGiven !== null}
            onclick={() => handleFeedback('helpful')}
            title="Helpful (+0.05 importance)"
          >
            <ThumbsUp size={14} strokeWidth={2} />
          </button>
          <button
            class="feedback-btn down {feedbackGiven === 'unhelpful' ? 'active' : ''}"
            disabled={submittingFeedback || feedbackGiven !== null}
            onclick={() => handleFeedback('unhelpful')}
            title="Unhelpful (-0.10 importance)"
          >
            <ThumbsDown size={14} strokeWidth={2} />
          </button>
        </div>
        {#if feedbackDelta !== null}
          <span class="feedback-delta {feedbackDelta > 0 ? 'positive' : 'negative'}">
            {feedbackDelta > 0 ? '+' : ''}{(feedbackDelta * 100).toFixed(0)}%
          </span>
        {/if}
      </div>

      <div class="timeline-section">
        <div class="timeline-header">
          <h3>
            <Clock size={14} strokeWidth={2} class="timeline-icon" />
            Timeline ({timeline.length})
          </h3>
        </div>

        {#if timelineLoading}
          <div class="timeline-loading"><Spinner size={14} /> Loading events...</div>
        {:else if timeline.length === 0}
          <div class="no-timeline">
            <p>No timeline events.</p>
            <p class="sub">Event history appears when uteke-serve is connected.</p>
          </div>
        {:else}
          <!-- Collapsed: show only first 5 events. Expanded: show all. -->
          <div class="timeline-list">
            {#each (timelineExpanded ? timeline : timeline.slice(0, 5)) as evt (evt.id)}
              <div class="timeline-item">
                <div class="timeline-dot {eventColor(evt.event_type)}"></div>
                <div class="timeline-content">
                  <div class="timeline-top">
                    <span class="timeline-badge {eventColor(evt.event_type)}">{eventLabel(evt.event_type)}</span>
                    <span class="timeline-time" title={new Date(evt.created_at).toLocaleString()}>{timeAgo(evt.created_at)}</span>
                  </div>
                  {#if evt.event_data}
                    <div class="timeline-data">{evt.event_data}</div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>

          {#if timeline.length > 5}
            <button class="timeline-toggle" onclick={() => (timelineExpanded = !timelineExpanded)}>
              {timelineExpanded ? 'Show less' : `Show ${timeline.length - 5} more`}
            </button>
          {/if}
        {/if}
      </div>

      <div class="neighbors-section">
        <div class="neighbors-header">
          <h3><Link2 size={14} strokeWidth={2} class="conn-icon" /> Connected ({neighbors.length})</h3>
        </div>

        {#if neighbors.length === 0}
          <div class="no-neighbors">
            <p>No connections yet.</p>
            <p class="sub">Memories with shared tags will appear here.</p>
          </div>
        {:else}
          <div class="neighbor-list">
            {#each neighbors as n (n.id)}
              <div
                class="neighbor-card"
                role="button"
                tabindex="0"
                onclick={() => onneighborclick(n.id)}
                onkeydown={(e) => e.key === 'Enter' && onneighborclick(n.id)}
              >
                <div class="neighbor-top">
                  <span class="rel-badge {relColor(n.relationship)}">{n.relationship}</span>
                  {#if n.score !== null && n.score > 0}
                    <span class="rel-score">{(n.score * 100).toFixed(0)}% match</span>
                  {/if}
                </div>
                <div class="neighbor-content">{n.content.slice(0, 120)}</div>
                <div class="neighbor-bottom">
                  <div class="tags">
                    {#each n.tags.slice(0, 4) as t}<span class="tag">{t}</span>{/each}
                  </div>
                  {#if n.shared_tags.length > 0}
                    <div class="shared-tags">
                      {#each n.shared_tags.slice(0, 3) as st}<span class="shared-tag">{st}</span>{/each}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if showDeleteConfirm}
    <ConfirmDialog
      open={showDeleteConfirm}
      title="Delete memory?"
      message="This action cannot be undone."
      confirmLabel="Delete"
      danger={true}
      onconfirm={handleDelete}
      oncancel={() => (showDeleteConfirm = false)}
    />
  {/if}
</div>

<style>
  .memory-detail { height: 100vh; display: flex; flex-direction: column; }

  .detail-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding: 16px 24px 0; flex-shrink: 0; }
  .back-btn { padding: 6px 12px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .back-btn:hover { background: var(--bg-hover); }
  .back-btn kbd { font-family: var(--font-mono); font-size: 0.65rem; padding: 1px 4px; background: var(--bg-hover); border-radius: var(--radius-sm); opacity: 0.7; }
  .header-actions { display: flex; gap: 8px; }
  .edit-btn, .delete-btn { padding: 6px 12px; border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; font-size: 0.85rem; }
  .edit-btn { background: var(--bg-tertiary); color: var(--text-primary); }
  .edit-btn:hover { border-color: var(--accent); }
  .delete-btn { background: transparent; color: var(--red); border-color: var(--red); }
  .delete-btn:hover { background: var(--red); color: var(--bg-primary); }

  .content-text {
    font-family: var(--font-sans); font-size: 0.95rem; line-height: 1.6;
    color: var(--text-primary); white-space: pre-wrap; word-wrap: break-word;
    margin-bottom: 20px; padding: 16px; background: var(--bg-tertiary);
    border-radius: var(--radius-lg); border: 1px solid var(--border);
  }

  .meta-grid { display: flex; flex-direction: column; gap: 8px; }
  .meta-row { display: flex; align-items: flex-start; gap: 12px; font-size: 0.85rem; }
  .meta-label { min-width: 80px; color: var(--text-muted); text-transform: uppercase; font-size: 0.7rem; letter-spacing: 0.5px; padding-top: 2px; }

  .tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.75rem; padding: 2px 8px; background: var(--bg-hover); color: var(--text-secondary); border-radius: var(--radius-sm); }

  .detail-body { overflow-y: auto; flex: 1; min-height: 0; padding: 0 24px 24px; }

  .docs-section { margin-top: 20px; }
  .docs-header { margin-bottom: 8px; }
  .docs-header h3 { font-size: 0.95rem; color: var(--text-secondary); display: inline-flex; align-items: center; gap: 6px; }
  .docs-header :global(.doc-icon) { stroke: var(--text-secondary); }
  .doc-list { display: flex; flex-direction: column; gap: 4px; }
  .doc-link {
    display: inline-flex; align-items: center; gap: 6px;
    font-size: 0.85rem; color: var(--accent); text-decoration: none;
    padding: 6px 10px; background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: var(--radius-sm); cursor: pointer; transition: border-color 0.1s;
    font-family: var(--font-mono);
  }
  .doc-link:hover { border-color: var(--accent); }
  .no-docs { text-align: center; padding: 12px; color: var(--text-muted); font-size: 0.85rem; }

  .feedback-section {
    display: flex; align-items: center; gap: 10px;
    margin-top: 16px; padding: 10px 14px;
    background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .feedback-label { font-size: 0.8rem; color: var(--text-muted); }
  .feedback-buttons { display: flex; gap: 6px; }
  .feedback-btn {
    display: flex; align-items: center; justify-content: center;
    width: 30px; height: 30px; border-radius: var(--radius-sm);
    border: 1px solid var(--border); background: transparent;
    color: var(--text-secondary); cursor: pointer; transition: all 0.15s;
  }
  .feedback-btn:disabled { opacity: 0.5; cursor: default; }
  .feedback-btn.up.active { background: var(--color-green-bg); color: var(--green); border-color: var(--green); }
  .feedback-btn.down.active { background: var(--color-red-bg, rgba(255,0,0,0.1)); color: var(--red); border-color: var(--red); }
  .feedback-btn:not(:disabled):hover { border-color: var(--accent); color: var(--text-primary); }
  .feedback-delta { font-size: 0.75rem; font-weight: 600; font-variant-numeric: tabular-nums; }
  .feedback-delta.positive { color: var(--green); }
  .feedback-delta.negative { color: var(--red); }

  /* Timeline */
  .timeline-section {
    margin-top: 20px; padding: 14px 16px;
    background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .timeline-header { margin-bottom: 12px; }
  .timeline-header h3 { font-size: 0.95rem; color: var(--text-secondary); display: inline-flex; align-items: center; gap: 6px; }
  .timeline-header :global(.timeline-icon) { stroke: var(--text-secondary); }

  .timeline-loading { display: flex; align-items: center; gap: 6px; color: var(--text-muted); font-size: 0.85rem; padding: 8px 0; }

  .timeline-list { display: flex; flex-direction: column; gap: 0; position: relative; }
  /* Vertical line */
  .timeline-list::before {
    content: ''; position: absolute; left: 5px; top: 6px; bottom: 6px; width: 1px;
    background: var(--border);
  }

  .timeline-item { display: flex; gap: 12px; padding: 6px 0; position: relative; }
  .timeline-dot {
    width: 11px; height: 11px; border-radius: 50%; flex-shrink: 0;
    margin-top: 3px; border: 2px solid var(--bg-tertiary); position: relative; z-index: 1;
  }
  /* Dot colors per event type */
  .timeline-dot.green  { background: var(--green); }
  .timeline-dot.blue   { background: var(--accent); }
  .timeline-dot.mauve  { background: var(--mauve); }
  .timeline-dot.yellow { background: var(--yellow); }
  .timeline-dot.peach  { background: var(--peach, #fab387); }
  .timeline-dot.red    { background: var(--red); }
  .timeline-dot.teal   { background: var(--teal); }

  .timeline-content { flex: 1; min-width: 0; }
  .timeline-top { display: flex; align-items: center; justify-content: space-between; gap: 8px; }

  .timeline-badge {
    font-size: 0.65rem; padding: 1px 6px; border-radius: var(--radius-sm);
    text-transform: uppercase; font-weight: 600; letter-spacing: 0.3px;
  }
  .timeline-badge.green  { background: var(--color-green-bg); color: var(--green); }
  .timeline-badge.blue   { background: var(--color-blue-bg); color: var(--accent); }
  .timeline-badge.mauve  { background: var(--color-mauve-bg); color: var(--mauve); }
  .timeline-badge.yellow { background: var(--color-yellow-bg); color: var(--yellow); }
  .timeline-badge.peach  { background: rgba(250, 179, 135, 0.15); color: var(--peach, #fab387); }
  .timeline-badge.red    { background: var(--color-red-bg, rgba(255,0,0,0.1)); color: var(--red); }
  .timeline-badge.teal   { background: var(--color-teal-bg); color: var(--teal); }

  .timeline-time {
    font-size: 0.7rem; color: var(--text-muted); white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .timeline-data {
    font-size: 0.75rem; color: var(--text-muted); margin-top: 2px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .timeline-toggle {
    width: 100%; margin-top: 8px; padding: 4px 8px;
    background: transparent; border: none; color: var(--text-muted);
    font-size: 0.8rem; cursor: pointer; text-align: center;
    transition: color 0.1s;
  }
  .timeline-toggle:hover { color: var(--accent); }

  .no-timeline { text-align: center; padding: 12px; color: var(--text-muted); font-size: 0.85rem; }
  .no-timeline .sub { font-size: 0.8rem; opacity: 0.7; margin-top: 4px; }

  .neighbors-section { margin-top: 24px; border-top: 1px solid var(--border); padding-top: 16px; }
  .neighbors-header { margin-bottom: 12px; }
  .neighbors-header h3 { font-size: 0.95rem; color: var(--text-secondary); display: inline-flex; align-items: center; gap: 6px; }
  .neighbors-header :global(.conn-icon) { stroke: var(--text-secondary); }

  .neighbor-list { display: flex; flex-direction: column; gap: 6px; }

  .neighbor-card {
    padding: 10px 14px; background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: var(--radius-md); cursor: pointer; transition: border-color 0.1s;
  }
  .neighbor-card:hover { border-color: var(--accent); }

  .neighbor-top { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 4px; }

  .rel-badge {
    font-size: 0.65rem; padding: 1px 6px; border-radius: var(--radius-sm); text-transform: uppercase;
    font-weight: 600; letter-spacing: 0.3px;
  }
  .rel-badge.refs { background: var(--color-blue-bg); color: var(--accent); }
  .rel-badge.super { background: var(--color-yellow-bg); color: var(--yellow); }
  .rel-badge.reply { background: var(--color-green-bg); color: var(--green); }
  .rel-badge.shared { background: var(--color-mauve-bg); color: var(--mauve); }
  .rel-badge.sim { background: var(--color-teal-bg); color: var(--teal); }
  .rel-badge.related { background: var(--bg-hover); color: var(--text-muted); }

  .rel-score { font-size: 0.65rem; color: var(--text-muted); }

  .neighbor-content { font-size: 0.85rem; color: var(--text-primary); margin-bottom: 6px; line-height: 1.4; }

  .neighbor-bottom { display: flex; justify-content: space-between; gap: 8px; }
  .shared-tags { display: flex; gap: 3px; }
  .shared-tag { font-size: 0.6rem; padding: 1px 4px; background: var(--color-mauve-bg); color: var(--mauve); border-radius: var(--radius-pill); }

  .no-neighbors { text-align: center; padding: 24px; color: var(--text-muted); }
  .no-neighbors .sub { font-size: 0.8rem; opacity: 0.7; margin-top: 4px; }

  .loading, .empty { text-align: center; padding: 40px; color: var(--text-muted); }
  .loading { display: flex; align-items: center; justify-content: center; gap: 8px; }
</style>
