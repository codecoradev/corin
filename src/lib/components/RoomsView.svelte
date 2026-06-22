<script lang="ts">
  import { uteke } from '../ts/ipc';
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
    if (utekeReady) {
      roomMemories = await uteke.roomRecall(roomId, 50).catch(() => []);
    }
  }
</script>

<div class="rooms-view">
  <div class="rooms-header">
    <h2>Rooms</h2>
    <span class="count">{rooms.length} rooms</span>
  </div>

  {#if loading}
    <div class="msg">Loading...</div>
  {:else if rooms.length === 0}
    <div class="msg">
      {#if utekeReady}
        <p>No rooms yet.</p>
        <p class="sub">Rooms are shared workspaces for multi-agent collaboration.<br>
        Create one via Uteke CLI: <code>uteke room create --id "sprint-1"</code></p>
      {:else}
        <p>Uteke not installed.</p>
        <p class="sub">Install from https://github.com/codecoradev/uteke</p>
      {/if}
    </div>
  {:else}
    <div class="layout">
      <div class="room-list">
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
            </div>
            <div class="room-ns">{room.namespace}</div>
          </button>
        {/each}
      </div>

      <div class="room-detail">
        {#if !selectedRoom}
          <div class="msg"><p>Select a room to view its memories</p></div>
        {:else}
          {@const room = rooms.find(r => r.id === selectedRoom)}
          <div class="detail-header">
            <h3>{room?.title ?? selectedRoom}</h3>
            {#if room}<span class="badge">{room.memory_count}</span>{/if}
          </div>
          <div class="mem-list">
            {#each roomMemories as m (m.id)}
              <div
                class="mem-card"
                role="button"
                tabindex="0"
                onclick={() => onmemoryclick(m.id)}
                onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
              >
                <div class="mem-content">{m.content.slice(0, 150)}</div>
                <div class="mem-meta">
                  <div class="tags">{#each m.tags.slice(0, 4) as t}<span class="tag">{t}</span>{/each}</div>
                  {#if m.namespace}<span class="ns">{m.namespace}</span>{/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .rooms-view { height: 100%; display: flex; flex-direction: column; }
  .rooms-header { padding: 16px 24px 8px; display: flex; align-items: baseline; gap: 12px; border-bottom: 1px solid var(--border); }
  h2 { font-size: 1.1rem; }
  .count { font-size: 0.8rem; color: var(--text-muted); }

  .layout { flex: 1; display: flex; overflow: hidden; }

  .room-list { width: 280px; overflow-y: auto; padding: 8px 12px; border-right: 1px solid var(--border); }

  .room-card { display: block; padding: 10px 14px; background: transparent; border: 1px solid transparent; border-radius: 6px; cursor: pointer; text-align: left; width: 100%; margin-bottom: 4px; }
  .room-card:hover { background: var(--bg-hover); }
  .room-card.active { background: var(--bg-hover); border-color: var(--accent); }

  .room-title { font-size: 0.9rem; color: var(--text-primary); margin-bottom: 2px; }
  .room-meta { font-size: 0.7rem; color: var(--text-muted); display: flex; gap: 4px; }
  .room-ns { font-size: 0.65rem; color: var(--accent); margin-top: 2px; }

  .room-detail { flex: 1; overflow-y: auto; padding: 16px 24px; }
  .detail-header { display: flex; align-items: center; gap: 8px; margin-bottom: 16px; }
  .detail-header h3 { font-size: 1rem; color: var(--accent); font-family: var(--font-mono); }
  .badge { font-size: 0.75rem; padding: 2px 8px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 10px; }

  .mem-list { display: flex; flex-direction: column; gap: 8px; }
  .mem-card { padding: 12px 16px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; }
  .mem-card:hover { border-color: var(--accent); }
  .mem-content { font-size: 0.85rem; color: var(--text-primary); line-height: 1.4; margin-bottom: 6px; }
  .mem-meta { display: flex; justify-content: space-between; gap: 8px; }
  .tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.7rem; padding: 2px 6px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 3px; }
  .ns { font-size: 0.7rem; padding: 2px 6px; background: rgba(137,180,250,0.15); color: var(--accent); border-radius: 3px; }

  .msg { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-muted); text-align: center; gap: 8px; }
  .msg .sub { font-size: 0.85rem; opacity: 0.7; line-height: 1.6; }
  .msg code { font-family: var(--font-mono); font-size: 0.8rem; padding: 2px 6px; background: var(--bg-tertiary); border-radius: 3px; color: var(--text-secondary); }
</style>
