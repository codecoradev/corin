<script lang="ts">
  import { memory as memoryApi, system, utekeServer } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    memory: MemoryEntry | null;
    namespace: string | null;
    onsave: () => void;
    onclose: () => void;
  }

  let { memory, namespace, onsave, onclose }: Props = $props();

  // Derive initial values reactively from props
  let content = $state('');
  let tagsInput = $state('');
  let contentType = $state('memory');
  let importance = $state(0.5);
  let ns = $state('');
  let namespaces = $state<string[]>([]);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let initialized = $state(false);
  let duplicateWarning = $state<{ content: string; score: number } | null>(null);
  let checkingDup = $state(false);

  // Initialize form when memory prop first becomes available
  $effect(() => {
    if (!initialized) {
      content = memory?.content ?? '';
      tagsInput = memory?.tags.join(', ') ?? '';
      contentType = memory?.content_type ?? 'memory';
      importance = memory?.importance ?? 0.5;
      ns = memory?.namespace ?? namespace ?? '';
      initialized = true;
    }
  });

  const contentTypes = ['memory', 'task', 'procedure', 'fact', 'decision'];

  async function loadNamespaces() {
    try {
      namespaces = await system.listNamespaces();
    } catch {
      namespaces = [];
    }
  }

  loadNamespaces();

  async function handleSave() {
    if (!content.trim()) {
      error = 'Content is required';
      return;
    }

    saving = true;
    error = null;

    try {
      const tags = tagsInput
        .split(',')
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

      // Check for duplicates via semantic search (if server available)
      try {
        const result = await utekeServer.remember(content, {
          tags,
          namespace: ns || undefined,
        });
        if (result.duplicate && !memory) {
          // Only block new memories, not edits
          duplicateWarning = {
            content: result.existing_content ?? '',
            score: result.score ?? 0,
          };
          saving = false;
          return;
        }
      } catch {
        // Server not available — fall through to Hub DB
      }

      if (memory) {
        // Edit existing: create new first, then delete old (avoid data loss on failure)
        const newId = await memoryApi.remember(content, {
          tags,
          content_type: contentType,
          importance,
          namespace: ns || undefined,
        });

        // Only delete old after successful creation
        if (newId) {
          await memoryApi.forget(memory.id);
        }
      } else {
        await memoryApi.remember(content, {
          tags,
          content_type: contentType,
          importance,
          namespace: ns || undefined,
        });
      }

      onsave();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) handleSave();
  }

  async function forceSave() {
    saving = true;
    error = null;
    try {
      const tags = tagsInput
        .split(',')
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

      if (memory) {
        const newId = await memoryApi.remember(content, {
          tags,
          content_type: contentType,
          importance,
          namespace: ns || undefined,
        });
        if (newId) await memoryApi.forget(memory.id);
      } else {
        await memoryApi.remember(content, {
          tags,
          content_type: contentType,
          importance,
          namespace: ns || undefined,
        });
      }
      onsave();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="modal-overlay"
  role="button"
  tabindex="0"
  onclick={onclose}
  onkeydown={(e) => e.key === 'Escape' && onclose()}
>
  <div class="editor-dialog" onclick={(e) => e.stopPropagation()} role="presentation">
    <div class="editor-header">
      <h2>{memory ? 'Edit Memory' : 'New Memory'}</h2>
      <button class="close-btn" onclick={onclose}>✕</button>
    </div>

    <div class="editor-body">
      <div class="field">
        <label for="content">Content</label>
        <textarea
          id="content"
          bind:value={content}
          placeholder="Write your memory..."
          rows="8"
        ></textarea>
      </div>

      <div class="field-row">
        <div class="field">
          <label for="tags">Tags (comma-separated)</label>
          <input
            id="tags"
            type="text"
            bind:value={tagsInput}
            placeholder="tag1, tag2, tag3"
          />
        </div>

        <div class="field">
          <label for="content-type">Content Type</label>
          <select id="content-type" bind:value={contentType}>
            {#each contentTypes as ct}
              <option value={ct}>{ct}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="field-row">
        <div class="field">
          <label for="namespace">Namespace</label>
          <input
            id="namespace"
            type="text"
            list="ns-list"
            bind:value={ns}
            placeholder="default"
          />
          <datalist id="ns-list">
            {#each namespaces as n}
              <option value={n}>{n}</option>
            {/each}
          </datalist>
        </div>

        <div class="field">
          <label for="importance">Importance: {(importance * 100).toFixed(0)}%</label>
          <input
            id="importance"
            type="range"
            min="0"
            max="1"
            step="0.1"
            bind:value={importance}
          />
        </div>
      </div>

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}

      {#if duplicateWarning}
        <div class="dup-warning">
          <div class="dup-header">⚠ Possible duplicate detected ({(duplicateWarning.score * 100).toFixed(0)}% match)</div>
          <div class="dup-content">{duplicateWarning.content.slice(0, 120)}</div>
          <div class="dup-actions">
            <button class="dup-cancel" onclick={async () => { duplicateWarning = null; await forceSave(); }}>Save anyway</button>
            <button class="dup-dismiss" onclick={() => { duplicateWarning = null; onclose(); }}>Cancel</button>
          </div>
        </div>
      {/if}
    </div>

    <div class="editor-footer">
      <span class="hint"><kbd>Ctrl+Enter</kbd> to save</span>
      <div class="footer-actions">
        <button class="cancel-btn" onclick={onclose}>Cancel</button>
        <button class="save-btn" onclick={handleSave} disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .editor-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 90%;
    max-width: 640px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .editor-header h2 {
    font-size: 1.1rem;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1.2rem;
    padding: 4px;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .editor-body {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
  }

  .field {
    margin-bottom: 16px;
    flex: 1;
  }

  .field-row {
    display: flex;
    gap: 16px;
  }

  .field label {
    display: block;
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .field textarea,
  .field input,
  .field select {
    width: 100%;
    padding: 8px 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.9rem;
    outline: none;
    font-family: inherit;
  }

  .field textarea {
    resize: vertical;
    line-height: 1.5;
  }

  .field input:focus,
  .field textarea:focus,
  .field select:focus {
    border-color: var(--accent);
  }

  .field input[type='range'] {
    padding: 0;
    height: 6px;
    -webkit-appearance: none;
    background: var(--bg-hover);
    border-radius: 3px;
    border: none;
  }

  .field input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    background: var(--accent);
    border-radius: 50%;
    cursor: pointer;
  }

  .error-msg {
    color: var(--red);
    font-size: 0.85rem;
    padding: 8px 12px;
    background: rgba(243, 139, 168, 0.1);
    border-radius: 4px;
  }

  .dup-warning {
    margin-top: 8px;
    padding: 12px;
    background: rgba(250, 179, 135, 0.1);
    border: 1px solid rgba(250, 179, 135, 0.3);
    border-radius: 6px;
  }

  .dup-header {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--accent);
    margin-bottom: 6px;
  }

  .dup-content {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin-bottom: 8px;
    padding: 6px 8px;
    background: var(--bg-primary);
    border-radius: 4px;
    font-style: italic;
  }

  .dup-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .dup-cancel,
  .dup-dismiss {
    padding: 4px 12px;
    font-size: 0.8rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    background: var(--bg-primary);
    color: var(--text-secondary);
  }

  .dup-cancel:hover {
    border-color: var(--accent);
  }

  .dup-dismiss:hover {
    border-color: var(--red);
  }

  .editor-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  kbd {
    padding: 1px 4px;
    background: var(--bg-hover);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.7rem;
  }

  .footer-actions {
    display: flex;
    gap: 8px;
  }

  .cancel-btn {
    padding: 8px 16px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }

  .save-btn {
    padding: 8px 20px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
  }

  .save-btn:disabled {
    opacity: 0.5;
  }

  .save-btn:not(:disabled):hover {
    opacity: 0.85;
  }
</style>
