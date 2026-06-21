<script lang="ts">
  import { uteke, memory as memoryApi } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
  }

  let { namespace, onmemoryclick }: Props = $props();

  let rooms = $state<{ name: string; count: number; tag: string }[]>([]);
  let loading = $state(true);
  let selectedRoom = $state<string | null>(null);
  let roomMemories = $state<MemoryEntry[]>([]);
  let utekeReady = $state(false);

  async function loadRooms() {
    loading = true;
    try {
      utekeReady = await uteke.available();

      // Get all memories, group by top tags
      const all: MemoryEntry[] = utekeReady
        ? await uteke.list({ namespace: namespace ?? undefined, limit: 200 })
        : await memoryApi.list({ namespace: namespace ?? undefined, limit: 200 });

      // Count tag frequency
      const tagCount = new Map<string, MemoryEntry[]>();
      for (const m of all) {
        for (const tag of m.tags) {
          if (!tagCount.has(tag)) tagCount.set(tag, []);
          tagCount.get(tag)!.push(m);
        }
      }

      // Top tags with 2+ memories = "rooms"
      rooms = Array.from(tagCount.entries())
        .filter(([_, mems]) => mems.length >= 2)
        .map(([tag, mems]) => ({ name: tag, tag, count: mems.length }))
        .sort((a, b) => b.count - a.count)
        .slice(0, 30);
    } catch {
      rooms = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    namespace;
    loadRooms();
  });

  async function selectRoom(tag: string) {
    selectedRoom = tag;
    try {
      const all = utekeReady
        ? await uteke.list({ namespace: namespace ?? undefined, tag, limit: 50 })
        : await memoryApi.list({ namespace: namespace ?? undefined, tag, limit: 50 });
      roomMemories = all;
    } catch {
      roomMemories = [];
    }
  }
</script>

<div class="rooms-view">
  <div class="rooms-header">
    <h2>Rooms</h2>
    <span class="rooms-count">{rooms.length} tags with 2+ memories</span>
  </div>

  {#if loading}
    <div class="center-msg">Loading...</div>
  {:else if rooms.length === 0}
    <div class="center-msg">
      <p>No rooms yet.</p>
      <p class="sub">Rooms are auto-generated from tags with 2+ memories.</p>
    </div>
  {:else}
    <div class="rooms-layout">
      <div class="room-list">
        {#each rooms as room (room.tag)}
          <button
            class="room-card"
            class:active={selectedRoom === room.tag}
            onclick={() => selectRoom(room.tag)}
          >
            <div class="room-icon">lami</div>
            <div class="room-info">
              <div class="room-name">{room.name}</div>
              <div class="room-meta">{room.count} memories</div>
            </div>
          </button>
        {/each}
      </div>

      <div class="room-detail">
        {#if !selectedRoom}
          <div class="center-msg">
            <p>Select a tag to view its memories</p>
          </div>
        {:else}
          <div class="room-detail-header">
            <h3>#{selectedRoom}</h3>
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
    width: 240px;
    overflow-y: auto;
    padding: 8px 12px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
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
    transition: background 0.1s;
  }

  .room-card:hover {
    background: var(--bg-hover);
  }

  .room-card.active {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .room-icon {
    font-size: 0.9rem;
    color: var(--accent);
    width: 20px;
    text-align: center;
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
