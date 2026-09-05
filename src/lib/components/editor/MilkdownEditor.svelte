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
</script>

<div class="milkdown-host" bind:this={mount}></div>

<style>
  .milkdown-host {
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
</style>
