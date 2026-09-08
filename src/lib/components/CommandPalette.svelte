<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { utekeServer } from '../ts/ipc';
  import type { UnifiedSearchResult, View } from '../ts/types';
  import { fadeQuick, overlayFade } from '../transitions';
  import { kbdCombo } from '../utils/platform';

  interface PaletteItem {
    key: string;
    icon: string;
    label: string;
    hint: string;
    kind: 'view' | 'action' | 'result';
    run: () => void;
  }

  let {
    onnavigate,
    onnewmemory,
    onopenmemory,
    onopendocument,
    onopensettings,
    onclose,
  }: {
    onnavigate: (v: View) => void;
    onnewmemory: () => void;
    onopenmemory: (id: string) => void;
    onopendocument: (slug: string) => void;
    onopensettings: () => void;
    onclose: () => void;
  } = $props();

  let query = $state('');
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLDivElement | null>(null);
  let results = $state<UnifiedSearchResult[]>([]);
  let searching = $state(false);
  let searchSeq = 0;

  // Debounced unified search (memories + documents) once the query is long enough.
  let debounce: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const q = query.trim();
    if (debounce) clearTimeout(debounce);
    if (q.length < 2) {
      results = [];
      searching = false;
      return;
    }
    searching = true;
    const seq = ++searchSeq;
    debounce = setTimeout(async () => {
      try {
        const hits = await utekeServer.recallUnified(q, { limit: 8 });
        if (seq === searchSeq) results = hits ?? [];
      } catch {
        if (seq === searchSeq) results = [];
      } finally {
        if (seq === searchSeq) searching = false;
      }
    }, 180);
  });

  // ── Static entries: views + actions ───────────────────────────────
  const viewItems: PaletteItem[] = [
    { key: 'v-dash', icon: '⌂', label: 'Go to Home', hint: 'Dashboard', kind: 'view', run: () => onnavigate('dashboard') },
    { key: 'v-mem', icon: '◉', label: 'Go to Memories', hint: 'List & hub', kind: 'view', run: () => onnavigate('memories') },
    { key: 'v-doc', icon: '▤', label: 'Go to Documents', hint: 'Docs', kind: 'view', run: () => onnavigate('documents') },
    { key: 'v-ns', icon: '◇', label: 'Go to Namespaces', hint: 'Workspaces', kind: 'view', run: () => onnavigate('namespaces') },
    { key: 'v-rooms', icon: '▣', label: 'Go to Rooms', hint: 'Agent rooms', kind: 'view', run: () => onnavigate('rooms') },
    { key: 'v-graph', icon: '⁂', label: 'Go to Graph', hint: 'Knowledge graph', kind: 'view', run: () => onnavigate('graph') },
    { key: 'v-life', icon: '♻', label: 'Go to Lifecycle', hint: 'Maintenance', kind: 'view', run: () => onnavigate('lifecycle') },
    { key: 'v-tools', icon: '⚒', label: 'Go to Tools', hint: 'Utilities', kind: 'view', run: () => onnavigate('tools') },
  ];

  const actionItems: PaletteItem[] = [
    { key: 'a-new', icon: '＋', label: 'New memory', hint: kbdCombo('N'), kind: 'action', run: () => onnewmemory() },
    { key: 'a-set', icon: '⚙', label: 'Open Settings', hint: 'Preferences', kind: 'action', run: () => onopensettings() },
  ];

  type Row =
    | { type: 'header'; label: string }
    | { type: 'item'; item: PaletteItem };

  const staticItems = $derived(
    [...viewItems, ...actionItems].filter((i) =>
      query.trim().length < 2 || i.label.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    const q = query.trim().toLowerCase();
    const views = staticItems.filter((i) => i.kind === 'view');
    const actions = staticItems.filter((i) => i.kind === 'action');
    if (views.length) {
      out.push({ type: 'header', label: 'Views' });
      for (const item of views) out.push({ type: 'item', item });
    }
    if (results.length) {
      out.push({ type: 'header', label: 'Search results' });
      for (const r of results) {
        out.push({
          type: 'item',
          item: {
            key: r.result_type === 'document' ? `d-${r.doc_slug}` : `m-${r.memory_id}`,
            icon: r.result_type === 'document' ? '▤' : '◉',
            label: r.result_type === 'document' ? (r.doc_title ?? r.doc_slug ?? 'document') : (r.content.slice(0, 70)),
            hint: r.result_type === 'document' ? 'Document' : 'Memory',
            kind: 'result',
            run: () => {
              if (r.result_type === 'document' && r.doc_slug) onopendocument(r.doc_slug);
              else if (r.memory_id) onopenmemory(r.memory_id);
            },
          },
        });
      }
    }
    if (actions.length) {
      out.push({ type: 'header', label: 'Actions' });
      for (const item of actions) out.push({ type: 'item', item });
    }
    return out;
  });

  const selectable = $derived(rows.filter((r): r is Extract<Row, { type: 'item' }> => r.type === 'item').map((r) => r.item));

  // Reset highlight when the list changes.
  $effect(() => {
    rows;
    activeIndex = Math.min(activeIndex, Math.max(0, selectable.length - 1));
  });

  function runItem(item: PaletteItem) {
    onclose();
    item.run();
  }

  function move(delta: number) {
    const n = selectable.length;
    if (!n) return;
    activeIndex = (activeIndex + delta + n) % n;
    tick().then(() => {
      listEl?.querySelector('[data-active="1"]')?.scrollIntoView({ block: 'nearest' });
    });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      move(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      move(-1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const item = selectable[activeIndex];
      if (item) runItem(item);
    }
  }

  onMount(() => {
    inputEl?.focus();
  });
</script>

<div
  class="palette-overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="palette" transition:fadeQuick={{ duration: 120 }} role="dialog" aria-label="Command palette" onkeydown={onKeydown}>
    <div class="palette-input-row">
      <span class="palette-glyph">⌘</span>
      <input
        bind:this={inputEl}
        bind:value={query}
        class="palette-input"
        type="text"
        placeholder="Search memories & documents, jump to a view, run an action…"
        spellcheck="false"
      />
      <kbd class="palette-esc">esc</kbd>
    </div>

    <div class="palette-list" bind:this={listEl}>
      {#each rows as row, i (row.type === 'header' ? `h-${row.label}` : row.item.key)}
        {#if row.type === 'header'}
          <div class="palette-header">{row.label}</div>
        {:else}
          {@const idx = selectable.indexOf(row.item)}
          <button
            class="palette-item"
            data-active={idx === activeIndex ? '1' : '0'}
            onmouseenter={() => (activeIndex = idx)}
            onclick={() => runItem(row.item)}
          >
            <span class="palette-item-icon">{row.item.icon}</span>
            <span class="palette-item-label">{row.item.label}</span>
            <span class="palette-item-hint">{row.item.hint}</span>
          </button>
        {/if}
      {:else}
        <div class="palette-empty">
          {searching ? 'Searching…' : query.trim().length >= 2 ? 'No matches.' : 'Type to search, or pick a destination.'}
        </div>
      {/each}
    </div>

    <div class="palette-footer">
      <span><kbd>↑</kbd><kbd>↓</kbd> navigate</span>
      <span><kbd>↵</kbd> open</span>
      <span><kbd>esc</kbd> close</span>
    </div>
  </div>
</div>

<style>
  .palette-overlay {
    position: fixed;
    inset: 0;
    /* Global surface: above every modal (editor 200, detail 103) so Cmd+K
       always takes over the screen cleanly instead of half-mixing with an
       open modal; below ui/Modal dialogs (1000) and toasts (2000). */
    z-index: 220;
    background: color-mix(in srgb, var(--bg-base, #0b0e14) 55%, transparent);
    backdrop-filter: blur(2px);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
  }
  .palette {
    width: min(640px, 92vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface, #11151d);
    border: 1px solid var(--border-subtle, #232a36);
    border-radius: 14px;
    box-shadow: 0 24px 64px rgb(0 0 0 / 45%);
    overflow: hidden;
  }
  .palette-input-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-subtle, #232a36);
  }
  .palette-glyph { color: var(--text-muted, #8b93a5); font-size: 15px; }
  .palette-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: var(--text-primary, #e6e9ef);
    font-size: 15px;
  }
  .palette-esc,
  .palette-footer kbd {
    font-family: inherit;
    font-size: 10px;
    color: var(--text-muted, #8b93a5);
    border: 1px solid var(--border-subtle, #232a36);
    border-radius: 4px;
    padding: 2px 5px;
  }
  .palette-list { overflow-y: auto; flex: 1; padding: 6px; }
  .palette-header {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted, #8b93a5);
    padding: 8px 10px 4px;
  }
  .palette-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: none;
    border: none;
    border-radius: 8px;
    color: var(--text-primary, #e6e9ef);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .palette-item[data-active='1'] { background: var(--bg-hover, #1a2130); }
  .palette-item-icon { width: 20px; text-align: center; color: var(--accent, #5eead4); }
  .palette-item-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .palette-item-hint { font-size: 11px; color: var(--text-muted, #8b93a5); }
  .palette-empty { padding: 22px; text-align: center; color: var(--text-muted, #8b93a5); font-size: 13px; }
  .palette-footer {
    display: flex;
    gap: 14px;
    padding: 8px 14px;
    border-top: 1px solid var(--border-subtle, #232a36);
    color: var(--text-muted, #8b93a5);
    font-size: 11px;
  }
</style>
