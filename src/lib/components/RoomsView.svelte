<script lang="ts">
  import { uteke, room } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface UtekeRoom {
    id: string;
    title: string | null;
    namespace: string;
    memory_count: number;
    participant_count: number;
    created_at: string;
    updated_at: string;
  }

  interface Participant {
    namespace: string;
    count: number;
  }

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
  }

  let { namespace, onmemoryclick }: Props = $props();

  let rooms = $state<UtekeRoom[]>([]);
  let loading = $state(true);
  let selectedRoom = $state<string | null>(null);
  let roomMemories = $state<MemoryEntry[]>([]);
  let utekeReady = $state(false);

  // Create room form state
  let showCreateForm = $state(false);
  let newName = $state('');
  let newNamespace = $state('');
  let creating = $state(false);

  // Tab state
  let activeTab = $state<'timeline' | 'summary' | 'participants'>('timeline');

  // Summary state
  let roomDocument = $state('');
  let documentLoading = $state(false);
  let documentLoaded = $state(false);

  // Confirm delete state
  let showDeleteConfirm = $state(false);

  function relativeTime(dateStr: string): string {
    if (!dateStr) return '';
    const now = Date.now();
    const then = new Date(dateStr).getTime();
    const diffMs = now - then;
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return 'just now';
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    if (diffDay < 30) return `${diffDay}d ago`;
    const diffMo = Math.floor(diffDay / 30);
    if (diffMo < 12) return `${diffMo}mo ago`;
    const diffYr = Math.floor(diffMo / 12);
    return `${diffYr}y ago`;
  }

  function getParticipants(): Participant[] {
    const counts: Record<string, number> = {};
    for (const m of roomMemories) {
      const ns = m.namespace ?? 'default';
      counts[ns] = (counts[ns] ?? 0) + 1;
    }
    return Object.entries(counts)
      .map(([namespace, count]) => ({ namespace, count }))
      .sort((a, b) => b.count - a.count);
  }

  async function loadRooms() {
    loading = true;
    try {
      utekeReady = await uteke.available();
      if (utekeReady) {
        rooms = await uteke.rooms(namespace ?? undefined);
      } else {
        rooms = [];
      }
    } catch {
      rooms = [];
    }
    loading = false;
  }

  $effect(() => {
    namespace;
    loadRooms();
  });

  async function selectRoom(roomId: string) {
    selectedRoom = roomId;
    roomMemories = [];
    activeTab = 'timeline';
    documentLoaded = false;
    roomDocument = '';
    if (utekeReady) {
      roomMemories = await uteke.roomRecall(roomId, 50).catch(() => []);
    }
  }

  async function loadDocument(roomId: string) {
    if (documentLoaded) return;
    documentLoading = true;
    try {
      roomDocument = await room.getDocument(roomId);
    } catch {
      roomDocument = '';
    }
    documentLoading = false;
    documentLoaded = true;
  }

  $effect(() => {
    if (selectedRoom && activeTab === 'summary') {
      loadDocument(selectedRoom);
    }
  });

  function toggleCreateForm() {
    showCreateForm = !showCreateForm;
    newName = '';
    newNamespace = '';
  }

  async function createRoom() {
    if (!newName.trim()) return;
    creating = true;
    try {
      await room.create(newName.trim(), {
        namespace: newNamespace.trim() || undefined,
      });
      showCreateForm = false;
      newName = '';
      newNamespace = '';
      await loadRooms();
    } catch {
      // Room creation failed silently — Tauri will log
    }
    creating = false;
  }

  function handleCreateKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      createRoom();
    } else if (e.key === 'Escape') {
      showCreateForm = false;
    }
  }

  // Basic markdown-ish formatting for room documents
  function formatMarkdown(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/^### (.+)$/gm, '<h4>$1</h4>')
      .replace(/^## (.+)$/gm, '<h3>$1</h3>')
      .replace(/^# (.+)$/gm, '<h2>$1</h2>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/`(.+?)`/g, '<code>$1</code>')
      .replace(/\n/g, '<br>');
  }
</script>

<div class="rooms-view">
  <div class="rooms-header">
    <div class="header-left">
      <h2>Rooms</h2>
      <span class="count">{rooms.length} rooms</span>
    </div>
    {#if utekeReady}
      <button class="btn-new" onclick={toggleCreateForm}>
        {showCreateForm ? '✕ Cancel' : '+ New Room'}
      </button>
    {/if}
  </div>

  {#if loading}
    <div class="msg">Loading...</div>
  {:else if rooms.length === 0}
    <div class="msg">
      {#if utekeReady}
        <p>No rooms yet.</p>
        <p class="sub">Rooms are shared workspaces for multi-agent collaboration.<br>
        Create one via Uteke CLI: <code>uteke room create --id "sprint-1"</code><br>
        Or click "New Room" above to create one from here.</p>
      {:else}
        <p>Uteke not installed.</p>
        <p class="sub">Install from https://github.com/codecoradev/uteke</p>
      {/if}
    </div>
  {:else}
    <div class="layout">
      <div class="room-list">
        <!-- Create room form (inline at top of list) -->
        {#if showCreateForm}
          <div class="create-form">
            <input
              type="text"
              class="input"
              placeholder="Room name"
              bind:value={newName}
              onkeydown={handleCreateKeydown}
              autofocus
            />
            <input
              type="text"
              class="input"
              placeholder="Namespace (optional)"
              bind:value={newNamespace}
              onkeydown={handleCreateKeydown}
            />
            <div class="create-actions">
              <button class="btn-create" onclick={createRoom} disabled={!newName.trim() || creating}>
                {creating ? 'Creating…' : 'Create'}
              </button>
              <button class="btn-cancel" onclick={toggleCreateForm}>Cancel</button>
            </div>
          </div>
        {/if}

        {#each rooms as room (room.id)}
          <button
            class="room-card"
            class:active={selectedRoom === room.id}
            onclick={() => selectRoom(room.id)}
          >
            <div class="room-title">{room.title ?? room.id}</div>
            <div class="room-meta">
              <span>{room.memory_count} memories</span>
              <span>·</span>
              <span>{room.participant_count} agents</span>
              <span>·</span>
              <span>{relativeTime(room.created_at)}</span>
            </div>
            <div class="room-ns">{room.namespace}</div>
          </button>
        {/each}
      </div>

      <div class="room-detail">
        {#if !selectedRoom}
          <div class="msg"><p>Select a room to view its memories</p></div>
        {:else}
          {@const currentRoom = rooms.find(r => r.id === selectedRoom)}
          <div class="detail-header">
            <h3>{currentRoom?.title ?? selectedRoom}</h3>
            {#if currentRoom}
              <span class="badge">{currentRoom.memory_count} memories</span>
            {/if}
            <div class="header-actions">
              {#if !showDeleteConfirm}
                <button
                  class="btn-delete"
                  disabled
                  title="Delete rooms via CLI: uteke room delete --id {selectedRoom}"
                >
                  🗑 Delete
                </button>
              {:else}
                <div class="delete-confirm">
                  <span class="delete-label">Delete "{currentRoom?.title ?? selectedRoom}"?</span>
                  <button class="btn-confirm-delete" disabled title="Not yet implemented — use CLI">Confirm</button>
                  <button class="btn-cancel-del" onclick={() => showDeleteConfirm = false}>Cancel</button>
                </div>
              {/if}
            </div>
          </div>

          <!-- Tabs -->
          <div class="tabs">
            <button class="tab" class:active={activeTab === 'timeline'} onclick={() => activeTab = 'timeline'}>Timeline</button>
            <button class="tab" class:active={activeTab === 'summary'} onclick={() => activeTab = 'summary'}>Summary</button>
            <button class="tab" class:active={activeTab === 'participants'} onclick={() => activeTab = 'participants'}>Participants</button>
          </div>

          <!-- Tab: Timeline -->
          {#if activeTab === 'timeline'}
            {#if roomMemories.length === 0}
              <div class="tab-empty">
                <p>No memories in this room yet.</p>
                <p class="sub">Memories will appear here when agents contribute to this room.<br>
                Add memories via CLI: <code>uteke remember --room {selectedRoom} "your content"</code></p>
              </div>
            {:else}
              <div class="mem-list">
                {#each roomMemories as m (m.id)}
                  <div
                    class="mem-card"
                    role="button"
                    tabindex="0"
                    onclick={() => onmemoryclick(m.id)}
                    onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
                  >
                    <div class="mem-timestamp">{relativeTime(m.created_at ?? '')}</div>
                    <div class="mem-content">{m.content.slice(0, 200)}{m.content.length > 200 ? '…' : ''}</div>
                    <div class="mem-meta">
                      <div class="tags">{#each m.tags.slice(0, 4) as t}<span class="tag">{t}</span>{/each}</div>
                      {#if m.namespace}<span class="ns">{m.namespace}</span>{/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}

          <!-- Tab: Summary -->
          {#if activeTab === 'summary'}
            {#if documentLoading}
              <div class="tab-loading">Loading document…</div>
            {:else if roomDocument}
              <div class="room-document">{@html formatMarkdown(roomDocument)}</div>
            {:else}
              <div class="tab-empty">
                <p>No summary available yet.</p>
                <p class="sub">Run a dream cycle to generate a structured document for this room.<br>
                <code>uteke dream</code></p>
              </div>
            {/if}
          {/if}

          <!-- Tab: Participants -->
          {#if activeTab === 'participants'}
            {@const participants = getParticipants()}
            {#if participants.length === 0}
              <div class="tab-empty">
                <p>No participants yet.</p>
                <p class="sub">Agents will appear here when they contribute memories to this room.</p>
              </div>
            {:else}
              <div class="participant-list">
                {#each participants as p (p.namespace)}
                  <div class="participant-card">
                    <div class="participant-info">
                      <span class="participant-ns">{p.namespace}</span>
                      <span class="participant-count">{p.count} memories</span>
                    </div>
                  </div>
                {/each}
                <div class="invite-section">
                  <button class="btn-invite" disabled title="Coming soon">
                    👤 Invite Agent (coming soon)
                  </button>
                </div>
              </div>
            {/if}
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .rooms-view { height: 100%; display: flex; flex-direction: column; }
  .rooms-header { padding: 16px 24px 8px; display: flex; align-items: baseline; justify-content: space-between; gap: 12px; border-bottom: 1px solid var(--border); }
  .header-left { display: flex; align-items: baseline; gap: 12px; }
  h2 { font-size: 1.1rem; }
  .count { font-size: 0.8rem; color: var(--text-muted); }

  .btn-new { font-size: 0.8rem; padding: 4px 12px; background: var(--accent); color: var(--bg-primary); border: none; border-radius: 6px; cursor: pointer; font-weight: 500; }
  .btn-new:hover { opacity: 0.85; }

  .layout { flex: 1; display: flex; overflow: hidden; }

  .room-list { width: 280px; overflow-y: auto; padding: 8px 12px; border-right: 1px solid var(--border); display: flex; flex-direction: column; }

  /* Create form */
  .create-form { padding: 10px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; margin-bottom: 8px; display: flex; flex-direction: column; gap: 8px; }
  .input { font-size: 0.85rem; padding: 6px 10px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px; outline: none; font-family: inherit; }
  .input:focus { border-color: var(--accent); }
  .input::placeholder { color: var(--text-muted); opacity: 0.6; }
  .create-actions { display: flex; gap: 8px; }
  .btn-create { font-size: 0.8rem; padding: 5px 14px; background: var(--accent); color: var(--bg-primary); border: none; border-radius: 4px; cursor: pointer; font-weight: 500; }
  .btn-create:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-cancel { font-size: 0.8rem; padding: 5px 14px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }
  .btn-cancel:hover { border-color: var(--text-muted); }

  /* Room cards */
  .room-card { display: block; padding: 10px 14px; background: transparent; border: 1px solid transparent; border-radius: 6px; cursor: pointer; text-align: left; width: 100%; margin-bottom: 4px; flex-shrink: 0; }
  .room-card:hover { background: var(--bg-hover); }
  .room-card.active { background: var(--bg-hover); border-color: var(--accent); }

  .room-title { font-size: 0.9rem; color: var(--text-primary); margin-bottom: 2px; }
  .room-meta { font-size: 0.7rem; color: var(--text-muted); display: flex; gap: 4px; }
  .room-ns { font-size: 0.65rem; color: var(--accent); margin-top: 2px; }

  /* Room detail */
  .room-detail { flex: 1; overflow-y: auto; padding: 16px 24px; display: flex; flex-direction: column; }
  .detail-header { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; flex-wrap: wrap; }
  .detail-header h3 { font-size: 1rem; color: var(--accent); font-family: var(--font-mono); }
  .badge { font-size: 0.75rem; padding: 2px 8px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 10px; }
  .header-actions { margin-left: auto; display: flex; align-items: center; gap: 6px; }

  .btn-delete { font-size: 0.75rem; padding: 3px 10px; background: transparent; color: var(--text-muted); border: 1px solid var(--border); border-radius: 4px; cursor: not-allowed; opacity: 0.5; }
  .btn-delete:disabled:hover { opacity: 0.5; }

  .delete-confirm { display: flex; align-items: center; gap: 6px; font-size: 0.75rem; }
  .delete-label { color: var(--text-secondary); }
  .btn-confirm-delete { font-size: 0.75rem; padding: 3px 10px; background: #e64553; color: #fff; border: none; border-radius: 4px; cursor: not-allowed; opacity: 0.5; }
  .btn-cancel-del { font-size: 0.75rem; padding: 3px 10px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }

  /* Tabs */
  .tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 16px; flex-shrink: 0; }
  .tab { font-size: 0.85rem; padding: 8px 16px; background: transparent; color: var(--text-muted); border: none; border-bottom: 2px solid transparent; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .tab:hover { color: var(--text-secondary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  /* Tab content */
  .tab-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; color: var(--text-muted); gap: 8px; padding: 48px 24px; }
  .tab-empty .sub { font-size: 0.85rem; opacity: 0.7; line-height: 1.6; }
  .tab-empty code { font-family: var(--font-mono); font-size: 0.8rem; padding: 2px 6px; background: var(--bg-tertiary); border-radius: 3px; color: var(--text-secondary); }
  .tab-loading { color: var(--text-muted); font-size: 0.85rem; padding: 32px; text-align: center; }

  /* Room document */
  .room-document { font-size: 0.85rem; color: var(--text-primary); line-height: 1.7; flex: 1; }
  .room-document :global(h2) { font-size: 1.05rem; color: var(--accent); margin: 16px 0 8px; }
  .room-document :global(h3) { font-size: 0.95rem; color: var(--accent); margin: 12px 0 6px; }
  .room-document :global(h4) { font-size: 0.9rem; color: var(--text-primary); margin: 10px 0 4px; font-weight: 600; }
  .room-document :global(code) { font-family: var(--font-mono); font-size: 0.8rem; padding: 1px 5px; background: var(--bg-tertiary); border-radius: 3px; color: var(--text-secondary); }

  /* Memory list (timeline) */
  .mem-list { display: flex; flex-direction: column; gap: 8px; }
  .mem-card { padding: 12px 16px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; }
  .mem-card:hover { border-color: var(--accent); }
  .mem-timestamp { font-size: 0.7rem; color: var(--text-muted); margin-bottom: 4px; }
  .mem-content { font-size: 0.85rem; color: var(--text-primary); line-height: 1.4; margin-bottom: 6px; }
  .mem-meta { display: flex; justify-content: space-between; gap: 8px; align-items: center; }
  .tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.7rem; padding: 2px 6px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 3px; }
  .ns { font-size: 0.7rem; padding: 2px 6px; background: rgba(137,180,250,0.15); color: var(--accent); border-radius: 3px; }

  /* Participants */
  .participant-list { display: flex; flex-direction: column; gap: 8px; }
  .participant-card { padding: 10px 14px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; }
  .participant-info { display: flex; justify-content: space-between; align-items: center; }
  .participant-ns { font-size: 0.9rem; color: var(--accent); font-family: var(--font-mono); }
  .participant-count { font-size: 0.75rem; color: var(--text-muted); }
  .invite-section { margin-top: 16px; padding-top: 16px; border-top: 1px solid var(--border); }
  .btn-invite { font-size: 0.8rem; padding: 6px 14px; background: var(--bg-hover); color: var(--text-muted); border: 1px dashed var(--border); border-radius: 6px; cursor: not-allowed; width: 100%; }

  /* General */
  .msg { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-muted); text-align: center; gap: 8px; }
  .msg .sub { font-size: 0.85rem; opacity: 0.7; line-height: 1.6; }
  .msg code { font-family: var(--font-mono); font-size: 0.8rem; padding: 2px 6px; background: var(--bg-tertiary); border-radius: 3px; color: var(--text-secondary); }
</style>
