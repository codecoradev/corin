<script lang="ts">
  import { kanban } from '../ts/ipc';

  interface Props {
    onclose: () => void;
    oncreated: () => void;
  }

  let { onclose, oncreated }: Props = $props();

  let title = $state('');
  let body = $state('');
  let assignee = $state('');
  let tenant = $state('');
  let priority = $state(0);
  let triage = $state(false);
  let submitting = $state(false);
  let error = $state<string | null>(null);

  async function handleSubmit() {
    if (!title.trim()) return;
    submitting = true;
    error = null;
    try {
      const result = await kanban.createTask({
        title: title.trim(),
        body: body.trim() || undefined,
        assignee: assignee.trim() || undefined,
        tenant: tenant.trim() || undefined,
        priority: priority || undefined,
        triage: triage || undefined,
      });
      if (result.warning) {
        error = result.warning;
      }
      oncreated();
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) handleSubmit();
  }
</script>

<div class="modal-overlay" onclick={onclose}>
  <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={handleKeydown}>
    <div class="modal-header">
      <h3>New Task</h3>
      <button class="modal-close" onclick={onclose}>✕</button>
    </div>

    <div class="modal-body">
      {#if error}
        <div class="modal-error">{error}</div>
      {/if}

      <label class="field">
        <span class="field-label">Title *</span>
        <input
          class="field-input"
          type="text"
          bind:value={title}
          placeholder="Task title..."
          autofocus
        />
      </label>

      <label class="field">
        <span class="field-label">Description</span>
        <textarea
          class="field-input field-textarea"
          rows="4"
          bind:value={body}
          placeholder="Optional description..."
        ></textarea>
      </label>

      <div class="field-row">
        <label class="field">
          <span class="field-label">Assignee</span>
          <input
            class="field-input"
            type="text"
            bind:value={assignee}
            placeholder="e.g. cto"
          />
        </label>

        <label class="field">
          <span class="field-label">Priority</span>
          <input
            class="field-input"
            type="number"
            bind:value={priority}
            min="0"
            max="10"
          />
        </label>
      </div>

      <div class="field-row">
        <label class="field">
          <span class="field-label">Tenant</span>
          <input
            class="field-input"
            type="text"
            bind:value={tenant}
            placeholder="Optional tenant"
          />
        </label>

        <label class="field field-checkbox">
          <input type="checkbox" bind:checked={triage} />
          <span class="field-label">Send to triage</span>
        </label>
      </div>
    </div>

    <div class="modal-footer">
      <button class="btn-cancel" onclick={onclose}>Cancel</button>
      <button
        class="btn-create"
        onclick={handleSubmit}
        disabled={!title.trim() || submitting}
      >
        {submitting ? 'Creating...' : 'Create Task'}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-sans);
  }

  .modal {
    width: 500px;
    max-width: 90vw;
    max-height: 90vh;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .modal-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px;
  }
  .modal-close:hover { color: var(--text-primary); }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .modal-error {
    background: rgba(243, 139, 168, 0.1);
    border: 1px solid var(--red);
    border-radius: var(--radius);
    padding: 8px 12px;
    font-size: 0.8rem;
    color: var(--red);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .field-row {
    display: flex;
    gap: 12px;
  }

  .field-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .field-input {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 0.85rem;
    padding: 8px 10px;
  }
  .field-input:focus { outline: none; border-color: var(--accent); }
  .field-input::placeholder { color: var(--text-muted); }

  .field-textarea {
    resize: vertical;
    min-height: 60px;
    font-family: var(--font-mono);
  }

  .field-checkbox {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    flex: 0;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  .btn-cancel {
    padding: 6px 16px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-cancel:hover { background: var(--bg-hover); }

  .btn-create {
    padding: 6px 16px;
    background: var(--accent);
    border: none;
    border-radius: var(--radius);
    color: var(--bg-primary);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .btn-create:disabled { opacity: 0.4; cursor: default; }
  .btn-create:hover:not(:disabled) { opacity: 0.85; }
</style>
