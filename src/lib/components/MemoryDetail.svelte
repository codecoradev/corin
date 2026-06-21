<script lang="ts">
  import { memory as memoryApi, graph } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    memoryId: string;
    onedit: (m: MemoryEntry) => void;
    onback: () => void;
    onneighborclick: (id: string) => void;
  }

  let { memoryId, onedit, onback, onneighborclick }: Props = $props();

  let memory = $state<MemoryEntry | null>(null);
  let neighbors = $state<MemoryEntry[]>([]);
  let loading = $state(true);
  let showDeleteConfirm = $state(false);

  async function load() {
    loading = true;
    try {
      memory = await memoryApi.get(memoryId);
      neighbors = await graph.getNeighbors(memoryId, 1);
    } catch {
      memory = null;
    } finally {
      loading = false;
    }
  }

  // Reload when memoryId changes (also runs on mount)
  $effect(() => {
    memoryId;
    load();
  });

  async function handleDelete() {
    await memoryApi.forget(memoryId);
    onback();
  }
</script>

<div class="memory-detail">
  <div class="detail-header">
    <button class="back-btn" onclick={onback}>← Back</button>
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
                {#each memory.tags as tag}
                  <span class="tag">{tag}</span>
                {/each}
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
          {#if memory.updated_at}
            <div class="meta-row">
              <span class="meta-label">Updated</span>
              <span>{new Date(memory.updated_at).toLocaleString()}</span>
            </div>
          {/if}
        </div>
      </div>

      {#if neighbors.length > 0}
        <div class="neighbors-section">
          <h3>Connected Memories ({neighbors.length})</h3>
          <div class="neighbor-list">
            {#each neighbors as n}
              <div
                class="neighbor-item"
                role="button"
                tabindex="0"
                onclick={() => onneighborclick(n.id)}
                onkeydown={(e) => e.key === 'Enter' && onneighborclick(n.id)}
              >
                <div class="neighbor-content">{n.content.slice(0, 80)}</div>
                <div class="neighbor-tags">
                  {#each n.tags.slice(0, 2) as tag}
                    <span class="tag">{tag}</span>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
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
  .memory-detail {
    padding: 16px 24px;
    max-width: 800px;
    margin: 0 auto;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .back-btn {
    padding: 6px 12px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }

  .back-btn:hover {
    background: var(--bg-hover);
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .edit-btn,
  .delete-btn {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .edit-btn {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .edit-btn:hover {
    border-color: var(--accent);
  }

  .delete-btn {
    background: transparent;
    color: var(--red);
    border-color: var(--red);
  }

  .delete-btn:hover {
    background: var(--red);
    color: var(--bg-primary);
  }

  .content-text {
    font-family: var(--font-sans);
    font-size: 0.95rem;
    line-height: 1.6;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-wrap: break-word;
    margin-bottom: 20px;
    padding: 16px;
    background: var(--bg-tertiary);
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  .meta-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .meta-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    font-size: 0.85rem;
  }

  .meta-label {
    min-width: 80px;
    color: var(--text-muted);
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.5px;
    padding-top: 2px;
  }

  .tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .tag {
    font-size: 0.75rem;
    padding: 2px 8px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 3px;
  }

  .neighbors-section {
    margin-top: 24px;
    border-top: 1px solid var(--border);
    padding-top: 16px;
  }

  .neighbors-section h3 {
    font-size: 0.95rem;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }

  .neighbor-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .neighbor-item {
    padding: 8px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.1s;
  }

  .neighbor-item:hover {
    border-color: var(--accent);
  }

  .neighbor-content {
    font-size: 0.85rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .neighbor-tags {
    margin-top: 4px;
    display: flex;
    gap: 4px;
  }

  .neighbor-tags .tag {
    font-size: 0.65rem;
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .confirm-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 24px;
    max-width: 360px;
  }

  .confirm-dialog h3 {
    margin-bottom: 8px;
  }

  .confirm-dialog p {
    color: var(--text-muted);
    margin-bottom: 16px;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .cancel-btn {
    padding: 6px 16px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }

  .confirm-delete-btn {
    padding: 6px 16px;
    background: var(--red);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
  }

  .loading,
  .empty {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
  }
</style>
