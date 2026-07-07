<script lang="ts">
  import { room } from '../../ts/ipc';

  interface Props {
    oncreated: () => void;
    oncancel: () => void;
    onerror: (op: string, e: unknown) => void;
  }

  let { oncreated, oncancel, onerror }: Props = $props();

  let newName = $state('');
  let newNamespace = $state('');
  let creating = $state(false);

  async function createRoom() {
    if (!newName.trim()) return;
    creating = true;
    try {
      await room.create(newName.trim(), {
        namespace: newNamespace.trim() || undefined,
      });
      newName = '';
      newNamespace = '';
      oncreated();
    } catch (e) {
      onerror('Create room', e);
    }
    creating = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      createRoom();
    } else if (e.key === 'Escape') {
      oncancel();
    }
  }
</script>

<div class="create-form">
  <input
    type="text"
    class="input"
    placeholder="Room name"
    bind:value={newName}
    onkeydown={handleKeydown}
    autofocus
  />
  <input
    type="text"
    class="input"
    placeholder="Namespace (optional)"
    bind:value={newNamespace}
    onkeydown={handleKeydown}
  />
  <div class="create-actions">
    <button class="btn-create" onclick={createRoom} disabled={!newName.trim() || creating}>
      {creating ? 'Creating…' : 'Create'}
    </button>
    <button class="btn-cancel" onclick={oncancel}>Cancel</button>
  </div>
</div>

<style>
  .create-form { padding: 10px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; margin-bottom: 8px; display: flex; flex-direction: column; gap: 8px; }
  .input { font-size: 0.85rem; padding: 6px 10px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px; outline: none; font-family: inherit; }
  .input:focus { border-color: var(--accent); }
  .input::placeholder { color: var(--text-muted); opacity: 0.6; }
  .create-actions { display: flex; gap: 8px; }
  .btn-create { font-size: 0.8rem; padding: 5px 14px; background: var(--accent); color: var(--bg-primary); border: none; border-radius: 4px; cursor: pointer; font-weight: 500; }
  .btn-create:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-cancel { font-size: 0.8rem; padding: 5px 14px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }
  .btn-cancel:hover { border-color: var(--text-muted); }
</style>
