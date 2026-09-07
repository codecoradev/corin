<script lang="ts">
  import { memory as memoryApi, uteke, utekeServer, graph as graphApi, memoryDocRefs, memoryFeedback, memoryTimeline, memoryUpdate } from '../ts/ipc';
  import type { MemoryEntry, TimelineEvent } from '../ts/types';
  import { X, Link2, FileText, ThumbsUp, ThumbsDown, Clock, Copy, Check, Sparkles, Link, Pin } from 'lucide-svelte';
  import { has as compatHas, minVersion } from '../ts/compat';
  import { ConfirmDialog, Spinner, toastStore } from '../ui';
  import { relativeTime } from '../utils/format';

  // Author identity — provenance slot is metadata.author (verified v0.16.0).
  const AUTHOR_COLORS = ['#7CB2FF', '#C4A7FF', '#4FD8D2', '#E89B3C', '#34D399', '#F87171', '#A78BFA', '#FBBF24'];
  function authorColor(name: string): string {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
    return AUTHOR_COLORS[h % AUTHOR_COLORS.length];
  }
  let author = $derived.by(() => {
    const meta = (memory as { metadata?: Record<string, unknown> } | null)?.metadata;
    const a = meta?.author;
    return typeof a === 'string' && a.trim() ? a.trim() : null;
  });
  let copiedId = $state(false);
  async function copyId() {
    try {
      await navigator.clipboard.writeText(memoryId);
      copiedId = true;
      setTimeout(() => (copiedId = false), 1500);
    } catch { /* clipboard unavailable */ }
  }

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
    /** Called after the memory moved to another namespace — refresh the list behind. */
    onmoved?: () => void;
  }

  let { memoryId, onedit, onback, onneighborclick, ondeleted, onmoved }: Props = $props();

  let memory = $state<MemoryEntry | null>(null);
  let neighbors = $state<Neighbor[]>([]);
  let docSlugs = $state<string[]>([]);
  let loading = $state(true);
  let showDeleteConfirm = $state(false);

  // Trust feedback state (#207)
  let feedbackGiven = $state<'helpful' | 'unhelpful' | null>(null);
  let feedbackDelta = $state<number | null>(null);
  let submittingFeedback = $state(false);

  // Namespace move — plain PUT /memory namespace field (uteke #1181, no re-embed).
  let nsList = $state<string[]>([]);
  let nsMoving = $state(false);
  let nsMoveAvailable = $state<boolean | null>(null);

  async function moveNamespace(target: string) {
    const m = memory;
    if (!m || !target || target === (m.namespace ?? '')) return;
    nsMoving = true;
    try {
      await memoryUpdate({ id: memoryId, namespace: target });
      memory = { ...m, namespace: target };
      toastStore.success(`Moved to ${target}`);
      onmoved?.();
    } catch (e) {
      toastStore.error(`Move failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      nsMoving = false;
    }
  }

  // Pin/unpin — PUT /memory `pinned` (supported since uteke 0.15; no gate).
  async function togglePin() {
    const m = memory;
    if (!m) return;
    const next = !m.pinned;
    try {
      await memoryUpdate({ id: memoryId, pinned: next });
      memory = { ...m, pinned: next };
      toastStore.success(next ? 'Memory pinned' : 'Memory unpinned');
    } catch (e) {
      toastStore.error(`Pin failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  // Timeline state
  let timeline = $state<TimelineEvent[]>([]);
  let timelineLoading = $state(false);
  let timelineExpanded = $state(false);
  // Related memories (semantic recall, #296)
  type RelatedHit = { id: string; content: string; score: number; tags: string[] };
  let related = $state<RelatedHit[]>([]);
  let relatedLoading = $state(false);
  // Suggested connections (#233): semantically related but not yet linked.
  let suggestions = $state<RelatedHit[]>([]);
  let linkingId = $state<string | null>(null);
  let linkedThisSession = $state<Set<string>>(new Set());
  let edgeWriteAvailable = $state<boolean | null>(null);

  async function load() {
    loading = true;
    try {
      try {
        memory = await memoryApi.get(memoryId);
      } catch {
        memory = await uteke.get(memoryId);
      }
      // Namespace list for the move control — non-fatal if unavailable.
      uteke.namespaces().then((ns) => (nsList = ns)).catch(() => (nsList = []));
      compatHas('namespaceMove').then((v) => (nsMoveAvailable = v === true)).catch(() => (nsMoveAvailable = false));
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
      // Related memories (#296): semantic recall seeded from this memory's
      // content — the reading-surface hero pattern from uteke-mobile.
      relatedLoading = true;
      utekeServer
        .recall(memory!.content.slice(0, 240), { limit: 6 })
        .then(async (hits) => {
          related = (hits ?? []).filter((h) => h.id !== memoryId).slice(0, 5);
          // Suggested connections (#233): semantically close (score >= 0.3)
          // but NOT explicitly linked in the graph — candidates for a new
          // explicit edge. (Semantic "neighbors" are computed, not edges.)
          const explicit = new Set<string>();
          try {
            const g = await graphApi.getData({ namespace: null });
            for (const e of g.edges) {
              if (e.source === memoryId) explicit.add(e.target);
              if (e.target === memoryId) explicit.add(e.source);
            }
          } catch {
            // graph endpoint unavailable — treat everything as unlinked
          }
          // Local embedding scores are low in absolute terms; keep the top
          // few (any score) that are not explicitly linked yet.
          suggestions = related.filter(
            (r) => r.score > 0 && !explicit.has(r.id) && !linkedThisSession.has(r.id),
          ).slice(0, 3);
        })
        .catch(() => (related = []))
        .finally(() => (relatedLoading = false));

      // Edge write needs uteke >= 0.16.1 (#1182); hide suggestions otherwise.
      compatHas('graphEdgeWrite').then((v) => (edgeWriteAvailable = v === true)).catch(() => (edgeWriteAvailable = false));

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
  // Create the explicit edge for a suggestion (#233)
  async function linkSuggestion(targetId: string) {
    linkingId = targetId;
    try {
      await graphApi.addEdge(memoryId, targetId, { edgeType: 'related', weight: 0.8 });
      linkedThisSession.add(targetId);
      suggestions = suggestions.filter((sg) => sg.id !== targetId);
      toastStore.success('Edge created');
      // Refresh neighbors so the new connection shows up
      neighbors = await uteke.neighbors(memoryId, 20).catch(() => neighbors);
    } catch (e) {
      toastStore.error(`Link failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      linkingId = null;
    }
  }
</script>

<div class="memory-detail">
  <div class="detail-header">
    <button class="back-btn" onclick={onback}><X size={13} strokeWidth={2} /> Close <kbd>Esc</kbd></button>
    {#if memory}
      <div class="header-actions">
        <button
          class="pin-btn"
          class:pinned={memory.pinned}
          onclick={togglePin}
          title={memory.pinned ? 'Unpin memory' : 'Pin memory'}
        >
          <Pin size={12} strokeWidth={2.25} />
          {memory.pinned ? 'Pinned' : 'Pin'}
        </button>
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
        {#if author || memory.created_at}
          <div class="author-head">
            {#if author}
              <span class="author-avatar" style="background: {authorColor(author)}">{author.trim()[0].toUpperCase()}</span>
              <span class="author-name">{author}</span>
            {/if}
            {#if memory.created_at}
              <span class="author-time" title={new Date(memory.created_at).toLocaleString()}>{relativeTime(memory.created_at)}</span>
            {/if}
          </div>
        {/if}
        <pre class="content-text">{memory.content}</pre>

        <div class="meta-grid">
          <div class="meta-row">
            <span class="meta-label">ID</span>
            <div class="id-row">
              <code class="id-full" title="Memory ID">{memoryId}</code>
              <button class="copyid-btn" onclick={copyId} title="Copy ID">
                {#if copiedId}<Check size={12} strokeWidth={2.5} />{:else}<Copy size={12} strokeWidth={2} />{/if}
              </button>
            </div>
          </div>
          {#if memory.tags.length > 0}
            <div class="meta-row">
              <span class="meta-label">Tags</span>
              <div class="tags">
                {#each memory.tags as tag}<span class="tag">{tag}</span>{/each}
              </div>
            </div>
          {/if}
          <div class="meta-row">
            <span class="meta-label">Namespace</span>
            {#if nsMoveAvailable && nsList.length > 0}
              <select
                class="ns-select"
                value={memory.namespace ?? ''}
                disabled={nsMoving}
                onchange={(e) => moveNamespace((e.currentTarget as HTMLSelectElement).value)}
                title="Move to another namespace"
              >
                {#if !memory.namespace}
                  <option value="">—</option>
                {:else if !nsList.includes(memory.namespace)}
                  <option value="">{memory.namespace}</option>
                {/if}
                {#each nsList as ns (ns)}
                  <option value={ns}>{ns}</option>
                {/each}
              </select>
            {:else}
              <span title={nsMoveAvailable === false ? `Namespace move needs uteke ≥ ${minVersion('namespaceMove')}` : undefined}>
                {memory.namespace ?? '—'}
              </span>
            {/if}
          </div>
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

      <div class="related-section">
        <div class="neighbors-header">
          <h3><Sparkles size={14} strokeWidth={2} class="conn-icon" /> Related</h3>
        </div>
        {#if relatedLoading}
          <div class="no-neighbors"><p>Searching…</p></div>
        {:else if related.length === 0}
          <div class="no-neighbors">
            <p>No related memories yet.</p>
            <p class="sub">Semantic matches appear as the library grows.</p>
          </div>
        {:else}
          <div class="neighbor-list">
            {#each related as r (r.id)}
              <div
                class="neighbor-card"
                role="button"
                tabindex="0"
                onclick={() => onneighborclick(r.id)}
                onkeydown={(e) => e.key === 'Enter' && onneighborclick(r.id)}
              >
                <div class="neighbor-top">
                  <span class="rel-badge related">semantic</span>
                  {#if r.score > 0}
                    <span class="rel-score">{(r.score * 100).toFixed(0)}% match</span>
                  {/if}
                </div>
                {#if r.score > 0}
                  <div class="match-bar" aria-hidden="true">
                    <div class="match-bar-fill" style="width: {Math.min(100, Math.round(r.score * 100))}%"></div>
                  </div>
                {/if}
                <div class="neighbor-content">{r.content.slice(0, 120)}</div>
                <div class="neighbor-bottom">
                  <div class="tags">
                    {#each r.tags.slice(0, 4) as t}<span class="tag">{t}</span>{/each}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if edgeWriteAvailable !== false && suggestions.length > 0}
        <div class="neighbors-section suggested-section">
          <div class="neighbors-header">
            <h3><Link size={14} strokeWidth={2} class="conn-icon" /> Suggested connections ({suggestions.length})</h3>
          </div>
          <div class="neighbor-list">
            {#each suggestions as sg (sg.id)}
              <div class="neighbor-card suggested-card">
                <div class="neighbor-top">
                  <span class="rel-badge related">unlinked</span>
                  {#if sg.score > 0}
                    <span class="rel-score">{(sg.score * 100).toFixed(0)}% match</span>
                  {/if}
                </div>
                <div class="neighbor-content">{sg.content.slice(0, 120)}</div>
                <button
                  class="link-btn"
                  disabled={linkingId === sg.id}
                  onclick={() => linkSuggestion(sg.id)}
                >
                  {linkingId === sg.id ? 'Linking…' : 'Create edge'}
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

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

  /* ── Reading-surface author header (#296) ──────────────────────────── */
  .author-head {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-bottom: 14px;
  }
  .author-avatar {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.78rem;
    font-weight: 700;
    color: var(--bg-primary);
    flex-shrink: 0;
  }
  .author-name { font-size: 0.9rem; font-weight: 600; color: var(--text-primary); }
  .author-time { margin-left: auto; font-size: 0.75rem; color: var(--text-muted); font-family: var(--font-mono); }

  .id-row { display: flex; align-items: flex-start; gap: 8px; min-width: 0; }
  .id-full {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-secondary);
    word-break: break-all;
    user-select: all;
  }
  .copyid-btn {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    border-radius: var(--radius-sm);
  }
  .copyid-btn:hover { color: var(--accent); }
  .back-btn { padding: 6px 12px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .back-btn:hover { background: var(--bg-hover); }
  .back-btn kbd { font-family: var(--font-mono); font-size: 0.65rem; padding: 1px 4px; background: var(--bg-hover); border-radius: var(--radius-sm); opacity: 0.7; }
  .header-actions { display: flex; gap: 8px; }
  .edit-btn, .delete-btn, .pin-btn { padding: 6px 12px; border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; font-size: 0.85rem; }
  .pin-btn { background: var(--bg-tertiary); color: var(--text-secondary); display: inline-flex; align-items: center; gap: 5px; }
  .pin-btn:hover { border-color: var(--accent); color: var(--text-primary); }
  .pin-btn.pinned { background: var(--color-teal-bg); color: var(--accent); border-color: var(--accent); }
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
  .ns-select {
    padding: 2px 6px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.78rem;
    outline: none;
    max-width: 220px;
  }
  .ns-select:hover:not(:disabled) { border-color: var(--accent); color: var(--text-primary); }

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
  .suggested-card { position: relative; }
  .link-btn {
    margin-top: 6px;
    padding: 4px 10px;
    font-size: 0.7rem;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    background: transparent;
    color: var(--accent, #5eead4);
    cursor: pointer;
    transition: background 120ms ease;
  }
  .link-btn:hover:not(:disabled) { background: var(--bg-hover); }
  .link-btn:disabled { opacity: 0.5; cursor: wait; }
  .match-bar {
    height: 3px;
    border-radius: 2px;
    background: var(--bg-hover, #1a2130);
    margin: 4px 0 2px;
    overflow: hidden;
  }
  .match-bar-fill {
    height: 100%;
    border-radius: 2px;
    background: var(--accent, #5eead4);
    opacity: 0.75;
  }

  .neighbor-content { font-size: 0.85rem; color: var(--text-primary); margin-bottom: 6px; line-height: 1.4; }

  .neighbor-bottom { display: flex; justify-content: space-between; gap: 8px; }
  .shared-tags { display: flex; gap: 3px; }
  .shared-tag { font-size: 0.6rem; padding: 1px 4px; background: var(--color-mauve-bg); color: var(--mauve); border-radius: var(--radius-sm); }

  .no-neighbors { text-align: center; padding: 24px; color: var(--text-muted); }
  .no-neighbors .sub { font-size: 0.8rem; opacity: 0.7; margin-top: 4px; }

  .loading, .empty { text-align: center; padding: 40px; color: var(--text-muted); }
  .loading { display: flex; align-items: center; justify-content: center; gap: 8px; }
</style>
