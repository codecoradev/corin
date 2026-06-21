<script lang="ts">
  import { uteke, memory as memoryApi } from '../ts/ipc';
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

  // Two modes: Uteke rooms (actual) or tag-based (fallback)
  let utekeRooms = $state<UtekeRoom[]>([]);
  let tagRooms = $state<{ name: string; count: number }[]>([]);
  let loading = $state(true);
  let selectedRoom = $state<string | null>(null);
  let selectedTag = $state<string | null>(null);
  let roomMemories = $state<MemoryEntry[]>([]);
  let utekeReady = $state(false);

  async function loadRooms() {
    loading = true;
    selectedRoom = null;
    selectedTag = null;
    roomMemories = [];
    try {
      utekeReady = await uteke.available();

      if (utekeReady) {
        // 1. Try actual Uteke rooms first
        utekeRooms = await uteke.rooms(namespace ?? undefined);

        // 2. Also build tag-based "rooms" as fallback/supplement
        const all = await uteke.list({ namespace: namespace ?? undefined, limit: 200 });
        const tagCount = new Map<string, number>();
        for (const m of all) {
          for (const tag of m.tags) {
            tagCount.set(tag, (tagCount.get(tag) ?? 0) + 1);
          }
        }
        tagRooms = Array.from(tagCount.entries())
          .filter(([_, count]) => count >= 2)
          .map(([name, count]) => ({ name, count }))
          .sort((a, b) => b.count - a.count)
          .slice(0, 30);
      } else {
        // Hub DB only
        const all = await memoryApi.list({ namespace: namespace ?? undefined, limit: 200 });
        const tagCount = new Map<string, number>();
        for (const m of all) {
          for (const tag of m.tags) {
            tagCount.set(tag, (tagCount.get(tag) ?? 0) + 1);
          }
        }
        tagRooms = Array.from(tagCount.entries())
          .filter(([_, count]) => count >= 2)
          .map(([name, count]) => ({ name, count }))
          .sort((a, b) => b.count - a.count)
          .slice(0, 30);
      }
    } catch {
      utekeRooms = [];
      tagRooms = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    namespace;
    loadRooms();
  });

  async function selectUtekeRoom(roomId: string) {
    selectedRoom = roomId;
    selectedTag = null;
    try {
      roomMemories = await uteke.roomRecall(roomId, 50);
    } catch {
      roomMemories = [];
    }
  }

  async function selectTagRoom(tag: string) {
    selectedTag = tag;
    selectedRoom = null;
    try {
      const src = utekeReady ? uteke : memoryApi;
      // @ts-expect-error - both have .list with tag support
      roomMemories = await src.list({
        namespace: namespace ?? undefined,
        tag,
        limit: 50,
      });
    } catch {
      roomMemories = [];
    }
  }
</script>

<div class="rooms-view">
  <div class="rooms-header">
    <h2>Rooms</h2>
    <span class="rooms-count">
      {utekeRooms.length + tagRooms.length} spaces
    </span>
  </div>

  {#if loading}
    <div class="center-msg">Loading...</div>
  {:else if utekeRooms.length === 0 && tagRooms.length === 0}
    <div class="center-msg">
      <p>No rooms yet.</p>
      <p class="sub">Rooms appear when memories share tags or are linked via Uteke rooms.</p>
    </div>
  {:else}
    <div class="rooms-layout">
      <div class="room-list">
        {#if utekeRooms.length > 0}
          <div class="section-label">Uteke Rooms</div>
          {#each utekeRooms as room (room.id)}
            <button
              class="room-card"
              class:active={selectedRoom === room.id}
              onclick={() => selectUtekeRoom(room.id)}
            >
              <div class="room-icon lami">lami</div>
              <div class="room-info">
                <div class="room-name">{room.title ?? room.id}</div>
                <div class="room-meta">
                  {room.memory_count} memories · {room.participant_count} participants
                </div>
              </div>
            </button>
          {/each}
        {/if}

        {#if tagRooms.length > 0}
          <div class="section-label">Tag Spaces</div>
          {#each tagRooms as room (room.name)}
            <button
              class="room-card"
              class:active={selectedTag === room.name}
              onclick={() => selectTagRoom(room.name)}
            >
              <div class="room-icon tag">#</div>
              <div class="room-info">
                <div class="room-name">{room.name}</div>
                <div class="room-meta">{room.count} memories</div>
              </div>
            </button>
          {/each}
        {/if}
      </div>

      <div class="room-detail">
        {#if !selectedRoom && !selectedTag}
          <div class="center-msg">
            <p>Select a room to view its memories</p>
          </div>
        {:else}
          <div class="room-detail-header">
            <h3>
              {#if selectedRoom}
                {utekeRooms.find((r) => r.id === selectedRoom)?.title ?? selectedRoom}
              {:else}
                #{selectedTag}
              {/if}
            </h3>
            <span class="badge">{roomMemories.length}</span>
          </div>
          <div class="room-memory-list">
            {#each roomMemories as m (m.id)}
              <div
                class="memory-card"
                role="button"
                tabindex="0"
                onclick={() => onmemoryclick(m.id)}
                onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
              >
                <div class="memory-content">{m.content.slice(0, 150)}</div>
                <div class="memory-meta">
                  <div class="tags">
                    {#each m.tags.slice(0, 4) as t}
                      <span class="tag">{t}</span>
                    {/each}
                  </div>
                  {#if m.namespace}
                    <span class="ns">{m.namespace}</span>
                  {/if}
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
  .rooms-view {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .rooms-header {
    padding: 16px 24px 8px;
    display: flex;
    align-items: baseline;
    gap: 12px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    font-size: 1.1rem;
  }

  .rooms-count {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .rooms-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .room-list {
    width: 260px;
    overflow-y: auto;
    padding: 8px 12px;
    border-right: 1px solid var(--border);
  }

  .section-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    padding: 8px 8px 4px;
    margin-top: 4px;
  }

  .room-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    width: 100%;
    margin-bottom: 2px;
  }

  .room-card:hover {
    background: var(--bg-hover);
  }

  .room-card.active {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .room-icon {
    font-size: 0.85rem;
    width: 24px;
    text-align: center;
    flex-shrink: 0;
  }

  .room-icon.lami {
    color: var(--teal);
  }

  .room-icon.tag {
    color: var(--mauve);
    font-weight: 700;
  }

  .room-info {
    flex: 1;
    min-width: 0;
  }

  .room-name {
    font-size: 0.85rem;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .room-meta {
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .room-detail {
    flex: 1;
    overflow-y: auto;
    padding: 16px 24px;
  }

  .room-detail-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
  }

  .room-detail-header h3 {
    font-size: 1rem;
    color: var(--accent);
    font-family: var(--font-mono);
  }

  .badge {
    font-size: 0.75rem;
    padding: 2px 8px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 10px;
  }

  .room-memory-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .memory-card {
    padding: 12px 16px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.1s;
  }

  .memory-card:hover {
    border-color: var(--accent);
  }

  .memory-content {
    font-size: 0.85rem;
    color: var(--text-primary);
    line-height: 1.4;
    margin-bottom: 6px;
  }

  .memory-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .tag {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: var(--bg-hover);
    color: var(--text-secondary);
    border-radius: 3px;
  }

  .ns {
    font-size: 0.7rem;
    padding: 2px 6px;
    background: rgba(137, 180, 250, 0.15);
    color: var(--accent);
    border-radius: 3px;
  }

  .center-msg {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    text-align: center;
    gap: 8px;
  }

  .center-msg .sub {
    font-size: 0.85rem;
    opacity: 0.7;
  }
</style>
