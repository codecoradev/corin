<script lang="ts">
  /**
   * Milkdown (Crepe) editor wrapper for Svelte 5 — issue #295.
   *
   * Markdown-native: editor state == markdown source, so what the agent
   * reads via /doc/get is exactly what the human sees.
   *
   * Reactivity contract (Cora-reviewed):
   * - The MOUNT effect reads only `mount` — value/placeholder are captured
   *   via untrack() so prop echoes never recreate the editor.
   * - `crepe` is $state: assigning it (post-create) re-runs the sync and
   *   readonly effects, so values arriving during pending-create are
   *   applied instead of silently dropped.
   */
  import { untrack } from 'svelte';
  import { Crepe } from '@milkdown/crepe';
  import { replaceAll } from '@milkdown/kit/utils';
  import '@milkdown/crepe/theme/common/style.css';
  import '@milkdown/crepe/theme/frame.css';

  interface Props {
    value?: string;
    onchange?: (markdown: string) => void;
    readonly?: boolean;
    placeholder?: string;
  }

  let { value = '', onchange, readonly = false, placeholder = 'Start writing… (/ for blocks)' }: Props = $props();

  let mount = $state<HTMLElement | null>(null);
  let crepe = $state<Crepe | null>(null);
  let applying = false; // suppresses markdownUpdated during programmatic replaceAll

  // Echo bookkeeping (plain vars — intentionally non-reactive):
  // lastEmitted = last markdown this component sent to the parent. When the
  // parent echoes it back via `value`, the sync effect skips — no async work
  // while typing, and no stale-apply races.
  let lastEmitted: string | null = null;
  let syncRun = 0; // generation counter: only the newest sync run may apply

  // Mount exactly once per element. NO reactive reads of value/placeholder here.
  $effect(() => {
    const el = mount;
    if (!el) return;
    let destroyed = false;
    const initialValue = untrack(() => value);
    const initialPlaceholder = untrack(() => placeholder);

    const c = new Crepe({
      root: el,
      defaultValue: initialValue,
      placeholder: initialPlaceholder,
      features: {
        [Crepe.Feature.LinkTooltip]: true,
        [Crepe.Feature.BlockEdit]: true,
        [Crepe.Feature.Placeholder]: true,
        [Crepe.Feature.Codeblock]: true,
        [Crepe.Feature.ListItemBlock]: true,
      },
      featureConfigs: {
        [Crepe.Feature.Codeblock]: { languagesWidget: true },
      },
    });

    c.on((listener: { markdownUpdated: (ctx: unknown, md: string) => void }) => {
      listener.markdownUpdated((_ctx, md) => {
        if (applying) return;
        lastEmitted = md;
        onchange?.(md);
        refreshSuggestions(md);
      });
    });

    c.create().then(() => {
      if (destroyed) return;
      // Assigning $state re-runs the sync/readonly effects below.
      crepe = c;
    });

    return () => {
      destroyed = true;
      if (crepe === c) crepe = null;
      c.destroy();
    };
  });

  // External value replacement (doc switch / restore). Re-runs when `value`
  // changes OR when `crepe` becomes available (pending-create window).
  // Own echoes short-circuit BEFORE any async work, so fast typing never
  // spawns racing reads; a generation counter discards superseded runs; and
  // try/finally guarantees `applying` is restored even if replaceAll throws.
  $effect(() => {
    const external = value;
    const c = crepe;
    if (!c) return;
    if (lastEmitted !== null && external === lastEmitted) return; // own echo
    const run = ++syncRun;
    void (async () => {
      let current: string;
      try {
        current = await c.getMarkdown();
      } catch {
        return;
      }
      if (run !== syncRun) return; // superseded by a newer sync run
      if (external === current || external === lastEmitted) return;
      applying = true;
      try {
        c.editor.action(replaceAll(external));
        lastEmitted = external;
      } finally {
        applying = false;
      }
    })();
  });

  // Readonly toggling — re-runs on crepe assignment or readonly change.
  $effect(() => {
    const c = crepe;
    const ro = readonly;
    if (c) c.setReadonly(ro);
  });
  // ── [[memory-link]] autocomplete (#295) ─────────────────────────────
  // Watches the markdown stream; when the user types "[[", opens a
  // suggestion popup fed by semantic recall. Selecting a suggestion
  // replaces the trailing "[[" with a uteke:// markdown link — plain
  // markdown, readable by agents via /doc/get with zero adapters.
  import { uteke, utekeServer } from '../../ts/ipc';

  interface MemorySuggestion {
    id: string;
    title: string;
  }
  let linkQuery = $state<string | null>(null); // null = popup closed
  let linkSuggestions = $state<MemorySuggestion[]>([]);
  let linkActive = $state(0);
  let linkStartPos = -1; // index of the "[[" opener in the markdown

  let linkDebounce: ReturnType<typeof setTimeout> | null = null;

  function detectLinkTrigger(md: string): { query: string; pos: number; openerLen: number } | null {
    // Milkdown escapes typed brackets: "[[" arrives in markdown as "\[\["
    // (4 chars). Find the LAST occurrence of either the escaped pair or the
    // raw pair, verifying the full opener pattern (not a lone bracket).
    // Runtime markdown for typed "[[": backslash bracket backslash bracket (4 chars).
    // Regex makes the escape count explicit instead of hand-counted in a literal.
    const ESCAPED_OPEN = /\\\[\\\[/g; // matches \\[ \\[ (1 backslash + bracket, twice)
    let lastEscaped = -1;
    for (const mm of md.matchAll(ESCAPED_OPEN)) lastEscaped = mm.index ?? -1;
    const rawIdx = md.lastIndexOf('[[');
    let idx = -1;
    let openerLen = 0;
    if (lastEscaped !== -1 && lastEscaped >= rawIdx) {
      idx = lastEscaped;
      openerLen = 4;
    } else if (rawIdx !== -1) {
      idx = rawIdx;
      openerLen = 2;
    }
    if (idx === -1) return null;
    const query = md.slice(idx + openerLen);
    if (query.includes(']]')) return null; // already closed
    if (query.length > 60) return null; // runaway opener
    return { query: query.toLowerCase(), pos: idx, openerLen };
  }

  function refreshSuggestions(md: string) {
    const hit = detectLinkTrigger(md);
    if (!hit) {
      linkQuery = null;
      linkSuggestions = [];
      return;
    }
    linkQuery = hit.query;
    linkStartPos = hit.pos;
    linkOpenerLen = hit.openerLen;
    if (linkDebounce) clearTimeout(linkDebounce);
    linkDebounce = setTimeout(async () => {
      try {
        const hits =
          hit.query.length >= 2
            ? await utekeServer.recall(hit.query, { limit: 6 })
            : await uteke.list({ limit: 6 }).then((ms) =>
                ms.map((m) => ({ id: m.id, content: m.content, score: 1, tags: m.tags ?? [] })),
              );
        linkSuggestions = (hits ?? [])
          .filter((h) => h.id)
          .map((h) => ({ id: h.id, title: h.content.slice(0, 60) }));
        linkActive = 0;
      } catch {
        linkSuggestions = [];
      }
    }, 150);
  }

  let linkOpenerLen = 2;

  function applySuggestion(s: MemorySuggestion) {
    if (linkStartPos < 0) return;
    const md = value;
    const replaced =
      md.slice(0, linkStartPos) +
      '[' + s.title + '](uteke://memory/' + s.id + ')' +
      md.slice(linkStartPos + linkOpenerLen + linkQuery!.length);
    applying = true;
    onchange?.(replaced);
    applying = false;
    linkQuery = null;
    linkSuggestions = [];
  }

  function closeSuggestions() {
    linkQuery = null;
    linkSuggestions = [];
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (linkQuery === null) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      linkActive = (linkActive + 1) % Math.max(1, linkSuggestions.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      linkActive = (linkActive - 1 + linkSuggestions.length) % Math.max(1, linkSuggestions.length);
    } else if (e.key === 'Enter' && linkSuggestions.length) {
      e.preventDefault();
      applySuggestion(linkSuggestions[linkActive]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeSuggestions();
    }
  }}
/>

<div class="milkdown-host" bind:this={mount}></div>

{#if linkQuery !== null}
  <div class="link-popup" role="listbox" aria-label="Memory link suggestions">
    <div class="link-popup-header">Link to memory</div>
    {#if linkSuggestions.length === 0}
      <div class="link-popup-empty">No matches — keep typing or Esc</div>
    {:else}
      {#each linkSuggestions as s, i (s.id)}
        <button
          class="link-option"
          data-active={i === linkActive ? '1' : '0'}
          role="option"
          aria-selected={i === linkActive}
          onmouseenter={() => (linkActive = i)}
          onclick={() => applySuggestion(s)}
        >
          <span class="link-option-glyph">◉</span>
          <span class="link-option-title">{s.title}</span>
        </button>
      {/each}
    {/if}
    <div class="link-popup-hint">↑↓ navigate · ↵ insert · Esc close</div>
  </div>
{/if}

<style>
  .link-popup {
    position: absolute;
    bottom: 12px;
    left: 12px;
    z-index: 30;
    width: min(380px, 90%);
    max-height: 260px;
    overflow-y: auto;
    background: var(--bg-surface, #11151d);
    border: 1px solid var(--border-subtle, #232a36);
    border-radius: 10px;
    box-shadow: 0 12px 32px rgb(0 0 0 / 40%);
    padding: 4px;
  }
  .link-popup-header {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted, #8b93a5);
    padding: 6px 8px 2px;
  }
  .link-option {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 6px 8px;
    background: none;
    border: none;
    border-radius: 6px;
    color: var(--text-primary, #e6e9ef);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .link-option[data-active='1'] { background: var(--bg-hover, #1a2130); }
  .link-option-glyph { color: var(--accent, #5eead4); }
  .link-option-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .link-popup-empty {
    padding: 10px 8px;
    font-size: 0.72rem;
    color: var(--text-muted, #8b93a5);
  }
  .link-popup-hint {
    padding: 6px 8px;
    border-top: 1px solid var(--border-subtle, #232a36);
    font-size: 0.6rem;
    color: var(--text-muted, #8b93a5);
  }
  .milkdown-host {
    position: relative;
    height: 100%;
    overflow-y: auto;
  }
  .milkdown-host :global(.ProseMirror) {
    outline: none;
    max-width: 72ch;
    margin: 0 auto;
    padding: 24px 8px 40vh;
    font-size: 0.92rem;
    line-height: 1.7;
  }
  /* Editor follows the app palette: Crepe's frame theme ships light-only
     tokens on .milkdown, so remap them onto the app's theme-aware tokens —
     one block covers dark AND light because the tokens flip via
     html[data-theme]. Specificity (0,2,0) beats the theme file's
     .milkdown (0,1,0). */
  .milkdown-host :global(.milkdown) {
    --crepe-color-background: var(--bg-primary);
    --crepe-color-on-background: var(--text-primary);
    --crepe-color-surface: var(--bg-secondary);
    --crepe-color-surface-low: var(--bg-tertiary);
    --crepe-color-on-surface: var(--text-primary);
    --crepe-color-on-surface-variant: var(--text-secondary);
    --crepe-color-outline: var(--border);
    --crepe-color-primary: var(--accent);
    --crepe-color-secondary: var(--bg-hover);
    --crepe-color-on-secondary: var(--text-primary);
    --crepe-color-inverse: var(--color-surface1);
    --crepe-color-on-inverse: var(--text-primary);
    --crepe-color-inline-code: var(--peach);
    --crepe-color-error: var(--red);
    --crepe-color-hover: var(--bg-hover);
    --crepe-color-selected: var(--color-teal-bg);
    --crepe-color-inline-area: var(--color-surface1);
  }
</style>
