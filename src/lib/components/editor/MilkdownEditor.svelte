<script lang="ts">
  import '@milkdown/crepe/theme/common/style.css';
  import '@milkdown/crepe/theme/frame.css';
  /**
   * Milkdown (Crepe) editor wrapper for Svelte 5 — issue #295.
   * Markdown-native: editor state == markdown source, so what the agent
   * reads via /doc/get is exactly what the human sees.
   */
  import { Crepe } from '@milkdown/crepe';
  import { replaceAll } from '@milkdown/kit/utils';

  interface Props {
    value?: string;
    onchange?: (markdown: string) => void;
    readonly?: boolean;
    placeholder?: string;
  }

  let { value = '', onchange, readonly = false, placeholder = 'Start writing… (/ for blocks)' }: Props = $props();

  let mount: HTMLElement | null = $state(null);
  let crepe: Crepe | null = null;
  let applying = false; // guard: parent echo of our own change

  $effect(() => {
    if (!mount) return;
    let destroyed = false;
    const c = new Crepe({
      root: mount,
      defaultValue: value,
      placeholder,
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
        if (!applying) onchange?.(md);
      });
    });
    c.create().then(() => {
      if (destroyed) return;
      crepe = c;
      if (readonly) c.setReadonly(true);
    });
    return () => {
      destroyed = true;
      crepe = null;
      c.destroy();
    };
  });

  // External value replacement (doc switch / restore)
  $effect(() => {
    const external = value;
    const c = crepe;
    if (!c) return;
    void (async () => {
      let current = '';
      try { current = await c.getMarkdown(); } catch { return; }
      if (external !== current) {
        applying = true;
        c.editor.action(replaceAll(external));
        applying = false;
      }
    })();
  });

  $effect(() => {
    crepe?.setReadonly(readonly);
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
