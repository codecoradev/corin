<script lang="ts">
  import { onMount } from 'svelte';
  import { room } from '../ts/ipc';
  import type { RoomEntry } from '../ts/types';

  interface Props {
    namespace: string | null;
    oncreateroom: () => void;
  }

  let { namespace, oncreateroom }: Props = $props();

  let rooms = $state<RoomEntry[]>([]);
  let loading = $state(true);
  let selectedRoom = $state<RoomEntry | null>(null);
  let document = $state<string | null>(null);
  let docLoading = $state(false);

  async function loadRooms() {
    loading = true;
    try {
      rooms = await room.list();
    } catch {
      rooms = [];
    } finally {
      loading = false;
    }
  }

  onMount(loadRooms);

  async function selectRoom(r: RoomEntry) {
    selectedRoom = r;
    docLoading = true;
    try {
      document = await room.getDocument(r.id);
    } catch {
      document = 'Failed to load room document.';
    } finally {
      docLoading = false;
    }
  }
</script>

<div class="rooms-view">
  <div class="rooms-toolbar">
    <h2>Rooms</h2>
    <button class="new-btn" onclick={oncreateroom}>+ New Room</button>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if rooms.length === 0}
    <div class="empty">
      <p>No rooms yet.</p>
      <p>Rooms are groups of memories with shared topics.</p>
    </div>
  {:else}
    <div class="rooms-layout">
      <div class="room-list">
        {#each rooms as r}
          <div
            class="room-card"
            class:active={selectedRoom?.id === r.id}
            role="button"
            tabindex="0"
            onclick={() => selectRoom(r)}
            onkeydown={(e) => e.key === 'Enter' && selectRoom(r)}
          >
            <div class="room-name">{r.name}</div>
            <div class="room-meta">
              <span>{r.memory_count} memories</span>
            </div>
          </div>
        {/each}
      </div>

      <div class="room-document">
        {#if !selectedRoom}
          <div class="empty">Select a room to view its document.</div>
        {:else if docLoading}
          <div class="loading">Loading...</div>
        {:else}
          <pre class="doc-content">{document}</pre>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .rooms-view {
    padding: 16px 24px;
  }

  .rooms-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  h2 {
    font-size: 1.1rem;
  }

  .new-btn {
    padding: 6px 12px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .rooms-layout {
    display: flex;
    gap: 16px;
    height: calc(100vh - 160px);
  }

  .room-list {
    width: 280px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
  }

  .room-card {
    padding: 10px 14px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
  }

  .room-card.active {
    border-color: var(--accent);
  }

  .room-card:hover {
    border-color: var(--accent);
  }

  .room-name {
    font-size: 0.9rem;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .room-meta {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .room-document {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    overflow-y: auto;
  }

  .doc-content {
    font-family: var(--font-sans);
    font-size: 0.9rem;
    line-height: 1.6;
    white-space: pre-wrap;
    word-wrap: break-word;
    color: var(--text-primary);
  }

  .loading,
  .empty {
    text-align: center;
    padding: 40px;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 8px;
  }
</style>
