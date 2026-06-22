<script lang="ts">
  import { uteke, system, memory as memoryApi } from '../ts/ipc';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    onmemoryclick: (id: string) => void;
  }

  let { onmemoryclick }: Props = $props();

  let namespaces = $state<{ name: string; count: number; source: string }[]>([]);
  let loading = $state(true);
  let selectedNs = $state<string | null>(null);
  let nsMemories = $state<MemoryEntry[]>([]);

  async function loadNamespaces() {
    loading = true;
    try {
      const hubNs = await system.listNamespaces().catch(() => []);
      const utekeOk = await uteke.available().catch(() => false);

      const result: { name: string; count: number; source: string }[] = [];
      for (const ns of hubNs) result.push({ name: ns, count: 0, source: 'hub' });

      if (utekeOk) {
        const utekeNs = await uteke.namespaces().catch(() => []);
        for (const ns of utekeNs) {
          const existing = result.find(r => r.name === ns);
          if (existing) existing.source = 'both';
          else result.push({ name: ns, count: 0, source: 'uteke' });
        }
        for (const item of result) {
          const mems = await uteke.list({ namespace: item.name, limit: 500 }).catch(() => []);
          item.count = mems.length;
        }
      }
      namespaces = result.sort((a, b) => b.count - a.count);
    } catch {
      namespaces = [];
    }
    loading = false;
  }

  $effect(() => {
    loadNamespaces();
  });

  async function selectNs(ns: string) {
    selectedNs = ns;
    nsMemories = [];
    const utekeOk = await uteke.available().catch(() => false);
    nsMemories = utekeOk
      ? await uteke.list({ namespace: ns, limit: 30 }).catch(() => [])
      : await memoryApi.list({ namespace: ns, limit: 30 }).catch(() => []);
  }
</script>

<div class="ns-view">
  <div class="ns-header">
    <h2>Namespaces</h2>
    <span class="count">{namespaces.length} namespaces</span>
  </div>

  {#if loading}
    <div class="msg">Loading...</div>
  {:else}
    <div class="layout">
      <div class="ns-list">
        {#each namespaces as ns (ns.name)}
          <button
            class="ns-card"
            class:active={selectedNs === ns.name}
            onclick={() => selectNs(ns.name)}
          >
            <div class="ns-name">{ns.name}</div>
            <div class="ns-meta">
              <span class="count">{ns.count}</span>
              <span class="src src-{ns.source}">{ns.source}</span>
            </div>
          </button>
        {/each}
      </div>

      <div class="ns-detail">
        {#if !selectedNs}
          <div class="msg">
            <p>Select a namespace to browse its memories</p>
          </div>
        {:else}
          <h3>{selectedNs}</h3>
          <div class="mem-list">
            {#each nsMemories as m (m.id)}
              <div
                class="mem-card"
                role="button"
                tabindex="0"
                onclick={() => onmemoryclick(m.id)}
                onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
              >
                <div class="mem-content">{m.content.slice(0, 120)}</div>
                <div class="mem-tags">
                  {#each m.tags.slice(0, 3) as t}<span class="tag">{t}</span>{/each}
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
  .ns-view { height: 100%; display: flex; flex-direction: column; }
  .ns-header { padding: 16px 24px 8px; display: flex; align-items: baseline; gap: 12px; border-bottom: 1px solid var(--border); }
  h2 { font-size: 1.1rem; }
  h3 { font-size: 0.95rem; color: var(--accent); font-family: var(--font-mono); margin-bottom: 12px; }
  .count { font-size: 0.8rem; color: var(--text-muted); }

  .layout { flex: 1; display: flex; overflow: hidden; }
  .ns-list { width: 260px; overflow-y: auto; padding: 8px 12px; border-right: 1px solid var(--border); }

  .ns-card { display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: transparent; border: 1px solid transparent; border-radius: 6px; cursor: pointer; text-align: left; width: 100%; margin-bottom: 2px; }
  .ns-card:hover { background: var(--bg-hover); }
  .ns-card.active { background: var(--bg-hover); border-color: var(--accent); }

  .ns-name { font-size: 0.85rem; color: var(--text-primary); }
  .ns-meta { display: flex; align-items: center; gap: 6px; }
  .src { font-size: 0.6rem; padding: 1px 5px; border-radius: 3px; text-transform: uppercase; }
  .src-uteke { background: rgba(148,226,213,0.15); color: var(--teal); }
  .src-hub { background: rgba(137,180,250,0.15); color: var(--accent); }
  .src-both { background: rgba(166,227,161,0.15); color: var(--green); }

  .ns-detail { flex: 1; overflow-y: auto; padding: 16px 24px; }
  .mem-list { display: flex; flex-direction: column; gap: 8px; }
  .mem-card { padding: 10px 14px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; }
  .mem-card:hover { border-color: var(--accent); }
  .mem-content { font-size: 0.85rem; color: var(--text-primary); margin-bottom: 4px; }
  .mem-tags { display: flex; gap: 4px; }
  .tag { font-size: 0.7rem; padding: 2px 6px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 3px; }

  .msg { flex: 1; display: flex; align-items: center; justify-content: center; color: var(--text-muted); text-align: center; }
</style>
