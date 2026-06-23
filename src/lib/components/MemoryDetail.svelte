<script lang="ts">
  import { memory as memoryApi, uteke, utekeServer } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

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
  }

  let { memoryId, onedit, onback, onneighborclick }: Props = $props();

  let memory = $state<MemoryEntry | null>(null);
  let neighbors = $state<Neighbor[]>([]);
  let loading = $state(true);
  let showDeleteConfirm = $state(false);

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
    } catch {
      memory = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    memoryId;
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
    } catch {
      await memoryApi.forget(memoryId);
    }
    onback();
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
</script>

<div class="memory-detail">
  <div class="detail-header">
    <button class="back-btn" onclick={onback}>✕ Close <kbd>Esc</kbd></button>
    {#if memory}
      <div class="header-actions">
        <button class="edit-btn" onclick={() => onedit(memory!)}>Edit</button>
        <button class="delete-btn" onclick={() => (showDeleteConfirm = true)}>Delete</button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
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

      <div class="neighbors-section">
        <div class="neighbors-header">
          <h3>🔗 Connected ({neighbors.length})</h3>
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
    <div
      class="modal-overlay"
      role="button"
      tabindex="0"
      onclick={() => (showDeleteConfirm = false)}
      onkeydown={(e) => e.key === 'Escape' && (showDeleteConfirm = false)}
    >
      <div class="confirm-dialog" onclick={(e) => e.stopPropagation()} role="presentation">
        <h3>Delete memory?</h3>
        <p>This action cannot be undone.</p>
        <div class="confirm-actions">
          <button class="cancel-btn" onclick={() => (showDeleteConfirm = false)}>Cancel</button>
          <button class="confirm-delete-btn" onclick={handleDelete}>Delete</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .memory-detail { padding: 16px 24px; }

  .detail-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
  .back-btn { padding: 6px 12px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .back-btn:hover { background: var(--bg-hover); }
  .back-btn kbd { font-family: var(--font-mono); font-size: 0.65rem; padding: 1px 4px; background: var(--bg-hover); border-radius: 3px; opacity: 0.7; }
  .header-actions { display: flex; gap: 8px; }
  .edit-btn, .delete-btn { padding: 6px 12px; border: 1px solid var(--border); border-radius: 4px; cursor: pointer; font-size: 0.85rem; }
  .edit-btn { background: var(--bg-tertiary); color: var(--text-primary); }
  .edit-btn:hover { border-color: var(--accent); }
  .delete-btn { background: transparent; color: var(--red); border-color: var(--red); }
  .delete-btn:hover { background: var(--red); color: var(--bg-primary); }

  .content-text {
    font-family: var(--font-sans); font-size: 0.95rem; line-height: 1.6;
    color: var(--text-primary); white-space: pre-wrap; word-wrap: break-word;
    margin-bottom: 20px; padding: 16px; background: var(--bg-tertiary);
    border-radius: 8px; border: 1px solid var(--border);
  }

  .meta-grid { display: flex; flex-direction: column; gap: 8px; }
  .meta-row { display: flex; align-items: flex-start; gap: 12px; font-size: 0.85rem; }
  .meta-label { min-width: 80px; color: var(--text-muted); text-transform: uppercase; font-size: 0.7rem; letter-spacing: 0.5px; padding-top: 2px; }

  .tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.75rem; padding: 2px 8px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 3px; }

  .neighbors-section { margin-top: 24px; border-top: 1px solid var(--border); padding-top: 16px; }
  .neighbors-header { margin-bottom: 12px; }
  .neighbors-header h3 { font-size: 0.95rem; color: var(--text-secondary); }

  .neighbor-list { display: flex; flex-direction: column; gap: 6px; }

  .neighbor-card {
    padding: 10px 14px; background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: 6px; cursor: pointer; transition: border-color 0.1s;
  }
  .neighbor-card:hover { border-color: var(--accent); }

  .neighbor-top { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 4px; }

  .rel-badge {
    font-size: 0.65rem; padding: 1px 6px; border-radius: 3px; text-transform: uppercase;
    font-weight: 600; letter-spacing: 0.3px;
  }
  .rel-badge.refs { background: rgba(137,180,250,0.15); color: var(--accent); }
  .rel-badge.super { background: rgba(249,226,175,0.15); color: var(--yellow); }
  .rel-badge.reply { background: rgba(166,227,161,0.15); color: var(--green); }
  .rel-badge.shared { background: rgba(203,166,247,0.15); color: var(--mauve); }
  .rel-badge.sim { background: rgba(148,226,213,0.15); color: var(--teal); }
  .rel-badge.related { background: var(--bg-hover); color: var(--text-muted); }

  .rel-score { font-size: 0.65rem; color: var(--text-muted); }

  .neighbor-content { font-size: 0.85rem; color: var(--text-primary); margin-bottom: 6px; line-height: 1.4; }

  .neighbor-bottom { display: flex; justify-content: space-between; gap: 8px; }
  .shared-tags { display: flex; gap: 3px; }
  .shared-tag { font-size: 0.6rem; padding: 1px 4px; background: rgba(203,166,247,0.15); color: var(--mauve); border-radius: 2px; }

  .no-neighbors { text-align: center; padding: 24px; color: var(--text-muted); }
  .no-neighbors .sub { font-size: 0.8rem; opacity: 0.7; margin-top: 4px; }

  .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 200; }
  .confirm-dialog { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 24px; max-width: 360px; }
  .confirm-dialog h3 { margin-bottom: 8px; }
  .confirm-dialog p { color: var(--text-muted); margin-bottom: 16px; }
  .confirm-actions { display: flex; gap: 8px; justify-content: flex-end; }
  .cancel-btn { padding: 6px 16px; background: var(--bg-tertiary); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }
  .confirm-delete-btn { padding: 6px 16px; background: var(--red); color: var(--bg-primary); border: none; border-radius: 4px; cursor: pointer; font-weight: 600; }

  .loading, .empty { text-align: center; padding: 40px; color: var(--text-muted); }
</style>
