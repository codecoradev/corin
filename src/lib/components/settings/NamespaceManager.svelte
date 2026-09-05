<script lang="ts">
  /**
   * Namespace management UI — issue #297.
   * Requires uteke >= 0.16.1 (#1183). On older servers the actions render
   * disabled with an honest upgrade tooltip (compat gate).
   */
  import { docs, memory as memoryApi } from '../../ts/ipc';
  import { has, minVersion } from '../../ts/compat';
  import { ConfirmDialog, Spinner, toastStore } from '../../ui';
  import { ArrowRightLeft, Trash2, Merge, ShieldAlert } from 'lucide-svelte';

  interface NamespaceRow {
    name: string;
    count?: number;
    active?: number;
    deprecated?: number;
  }

  interface Props {
    /** Namespace selected as default workspace (cannot be deleted). */
    protectedNamespace?: string;
  }

  let { protectedNamespace = 'default' }: Props = $props();

  let rows = $state<NamespaceRow[]>([]);
  let loading = $state(true);
  let nsSupported = $state<boolean | null>(null);

  // action state
  let moveFrom = $state<string | null>(null);
  let moveTo = $state('');
  let renameTarget = $state<string | null>(null);
  let renameTo = $state('');
  let deleteTarget = $state<string | null>(null);
  let deleteStrategy = $state<'refuse' | 'merge' | 'deprecate'>('refuse');
  let deleteMergeTarget = $state('');
  let busy = $state(false);

  async function load() {
    loading = true;
    try {
      nsSupported = (await has('namespaceManage')) ?? false;
      const raw = await docs.listNamespaces();
      // enrich with counts; breakdown fields only exist on 0.16.1+
      const enriched: NamespaceRow[] = [];
      for (const name of raw) {
        try {
          const s = await memoryApi.stats(name);
          enriched.push({ name, count: s.total ?? s.count ?? undefined });
        } catch {
          enriched.push({ name });
        }
      }
      rows = enriched;
    } catch {
      rows = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    load();
  });

  async function moveMemoryPrompt() {
    if (!moveFrom || !moveTo.trim()) return;
    busy = true;
    try {
      // Move = list all in namespace, PUT each. Bounded by pager caps client-side.
      toastStore.info('Moving memories…');
      // (bulk path uses /namespaces/rename which is a single atomic UPDATE)
      await rename(moveFrom, moveTo.trim());
      toastStore.success(`Moved to ${moveTo.trim()}`);
      moveFrom = null;
      moveTo = '';
      await load();
    } catch (e) {
      toastStore.error(`Move failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      busy = false;
    }
  }

  async function rename(from: string, to: string) {
    const res = await fetch('/api/namespaces/rename', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ from, to }),
    });
    if (!res.ok) throw new Error(`server returned ${res.status}`);
  }

  async function performRename() {
    if (!renameTarget || !renameTo.trim()) return;
    busy = true;
    try {
      await rename(renameTarget, renameTo.trim());
      toastStore.success(`Renamed to ${renameTo.trim()}`);
      renameTarget = null;
      renameTo = '';
      await load();
    } catch (e) {
      toastStore.error(`Rename failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      busy = false;
    }
  }

  async function performDelete() {
    if (!deleteTarget) return;
    if (deleteStrategy === 'merge' && !deleteMergeTarget.trim()) return;
    busy = true;
    try {
      const res = await fetch('/api/namespaces/delete', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: deleteTarget,
          strategy: deleteStrategy,
          target: deleteStrategy === 'merge' ? deleteMergeTarget.trim() : undefined,
        }),
      });
      const body = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(body?.error ?? `server returned ${res.status}`);
      toastStore.success(`${deleteTarget}: ${body?.affected ?? 0} memories processed`);
      deleteTarget = null;
      await load();
    } catch (e) {
      toastStore.error(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      busy = false;
    }
  }
</script>

<div class="ns-manager">
  <div class="ns-head">
    <h4>Namespaces</h4>
    {#if nsSupported === false}
      <span class="upgrade-hint" title="Namespace management needs uteke >= {minVersion('namespaceManage')}">
        <ShieldAlert size={12} strokeWidth={2} />
        needs uteke ≥ {minVersion('namespaceManage')}
      </span>
    {/if}
  </div>

  {#if loading}
    <div class="ns-loading"><Spinner size={16} /> Loading…</div>
  {:else}
    <table class="ns-table">
      <thead>
        <tr><th>Name</th><th>Active</th><th>Deprecated</th><th></th></tr>
      </thead>
      <tbody>
        {#each rows as row (row.name)}
          <tr>
            <td class="ns-name">{row.name}{row.name === protectedNamespace ? ' *' : ''}</td>
            <td class="mono">{row.active ?? row.count ?? '—'}</td>
            <td class="mono dim">{row.deprecated ?? '—'}</td>
            <td class="ns-actions">
              {#if nsSupported}
                <button
                  class="ns-btn"
                  title="Rename / merge"
                  onclick={() => { renameTarget = row.name; renameTo = ''; }}
                ><ArrowRightLeft size={12} strokeWidth={2} /></button>
                <button
                  class="ns-btn danger"
                  title="Delete (choose strategy)"
                  disabled={row.name === protectedNamespace}
                  onclick={() => { deleteTarget = row.name; deleteStrategy = 'refuse'; }}
                ><Trash2 size={12} strokeWidth={2} /></button>
              {:else}
                <button
                  class="ns-btn"
                  disabled
                  title="Butuh uteke ≥ {minVersion('namespaceManage')} (terpasang: versi lama)"
                ><ArrowRightLeft size={12} strokeWidth={2} /></button>
                <button class="ns-btn danger" disabled title="Butuh uteke ≥ {minVersion('namespaceManage')}"><Trash2 size={12} strokeWidth={2} /></button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="ns-note">* default workspace — protected</p>
  {/if}
</div>

{#if renameTarget}
  <ConfirmDialog
    open
    title="Rename or merge namespace?"
    confirmLabel={renameTo.trim() ? 'Apply' : ''}
    onconfirm={performRename}
    oncancel={() => (renameTarget = null)}
  >
    <div class="dialog-body">
      <p><code>{renameTarget}</code> →</p>
      <input class="ns-input" type="text" bind:value={renameTo} placeholder="new-name (existing name = merge)" />
      <p class="hint">If the target exists, all memories are merged into it (single atomic update, no data loss).</p>
    </div>
  </ConfirmDialog>
{/if}

{#if deleteTarget}
  <ConfirmDialog
    open
    title="Delete namespace — choose strategy"
    confirmLabel="Execute"
    onconfirm={performDelete}
    oncancel={() => (deleteTarget = null)}
  >
    <div class="dialog-body">
      <p>Namespace <code>{deleteTarget}</code> — memories must go somewhere:</p>
      <label class="strategy">
        <input type="radio" bind:group={deleteStrategy} value="refuse" />
        <span><b>Refuse</b> — cancel if any memory still uses it (safest)</span>
      </label>
      <label class="strategy">
        <input type="radio" bind:group={deleteStrategy} value="merge" />
        <span><b>Merge into</b> — move everything to:</span>
        {#if deleteStrategy === 'merge'}
          <input class="ns-input" type="text" bind:value={deleteMergeTarget} placeholder="target-namespace" />
        {/if}
      </label>
      <label class="strategy">
        <input type="radio" bind:group={deleteStrategy} value="deprecate" />
        <span><b>Deprecate</b> — soft-delete all (restorable from Recycle Bin until TTL)</span>
      </label>
      <p class="hint"><Merge size={11} strokeWidth={2} style="vertical-align:-1px" /> No hard delete exists — data is never destroyed.</p>
    </div>
  </ConfirmDialog>
{/if}

<style>
  .ns-manager { margin-bottom: 24px; }
  .ns-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; }
  .ns-head h4 { margin: 0; font-size: 0.9rem; }
  .upgrade-hint {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: 0.72rem; color: var(--text-muted);
    border: 1px solid var(--border); border-radius: var(--radius-pill);
    padding: 2px 9px;
  }
  .ns-loading { color: var(--text-muted); font-size: 0.8rem; display: flex; gap: 8px; align-items: center; }
  .ns-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
  .ns-table th {
    text-align: left; font-size: 0.64rem; letter-spacing: 0.09em;
    text-transform: uppercase; color: var(--text-muted);
    padding: 4px 8px; border-bottom: 1px solid var(--border);
  }
  .ns-table td { padding: 6px 8px; border-bottom: 1px solid var(--border); }
  .ns-name { font-family: var(--font-mono); color: var(--text-primary); }
  .mono { font-family: var(--font-mono); font-size: 0.75rem; }
  .dim { color: var(--text-muted); }
  .ns-actions { text-align: right; white-space: nowrap; }
  .ns-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 24px; margin-left: 4px;
    background: transparent; border: 1px solid var(--border);
    border-radius: var(--radius-sm); color: var(--text-secondary); cursor: pointer;
  }
  .ns-btn:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); }
  .ns-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .ns-btn.danger:hover:not(:disabled) { color: var(--red); border-color: var(--red); }
  .ns-note { font-size: 0.68rem; color: var(--text-muted); margin-top: 8px; }
  .dialog-body p { margin: 6px 0; }
  .dialog-body code { font-family: var(--font-mono); font-size: 0.8rem; background: var(--bg-tertiary); padding: 1px 6px; border-radius: var(--radius-sm); }
  .ns-input {
    width: 100%; padding: 6px 10px; margin: 6px 0;
    background: var(--bg-primary); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: var(--radius-md);
    font-family: var(--font-mono); font-size: 0.8rem;
  }
  .strategy { display: flex; align-items: center; gap: 8px; margin: 8px 0; font-size: 0.8rem; }
  .hint { font-size: 0.7rem; color: var(--text-muted); }
</style>
