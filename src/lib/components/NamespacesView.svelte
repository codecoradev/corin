<script lang="ts">
  import { docs, system, uteke, utekeServer } from '../ts/ipc';
  import { has as compatHas, minVersion } from '../ts/compat';
  import { ConfirmDialog, toastStore } from '../ui';
  import { ArrowRightLeft, Trash2 } from 'lucide-svelte';
  import { createPager } from '../stores/pagination.svelte';
  import type { MemoryEntry } from '../ts/types';

  interface Props {
    onmemoryclick: (id: string) => void;
  }

  let { onmemoryclick }: Props = $props();

  let namespaces = $state<{ name: string; count: number }[]>([]);
  let loading = $state(true);
  let selectedNs = $state<string | null>(null);

  // ── Management (rename/merge, delete) — same ops as Settings → Namespaces,
  // now inline where you browse. Gated on uteke >= 0.16.1 (#1183).
  let nsSupported = $state<boolean | null>(null);
  let defaultNs = $state('default');
  let busy = $state(false);

  let renameTarget = $state<string | null>(null);
  let renameTo = $state('');
  // Explicit namespace creation: uteke has no create endpoint — a namespace
  // comes into being together with its first memory, so we seed a small
  // placeholder the user can replace or delete.
  let showNewNs = $state(false);
  let newNsName = $state('');
  let deleteTarget = $state<string | null>(null);
  let deleteStrategy = $state<'refuse' | 'merge' | 'deprecate'>('refuse');
  let deleteMergeTarget = $state('');

  // Detail pager (paginated memories of the selected namespace).
  let detailPager = $state(createPager({ pageSize: 20 }));

  async function loadNamespaces() {
    loading = true;
    try {
      // Single call: namespace names + memory counts (uteke >= #527).
      const counted = await uteke.namespacesWithCounts();
      namespaces = counted
        .map((item) => ({ name: item.name, count: item.count }))
        .sort((a, b) => a.name.localeCompare(b.name));
    } catch {
      namespaces = [];
    } finally {
      loading = false;
    }
  }

  async function loadCompat() {
    nsSupported = (await compatHas('namespaceManage')) ?? false;
    try {
      const s = await system.getSettings();
      defaultNs = s['default_namespace'] || 'default';
    } catch { /* settings unavailable — protect 'default' */ }
  }

  $effect(() => {
    loadCompat();
    loadNamespaces();
  });

  function isProtected(name: string): boolean {
    return name === defaultNs;
  }

  async function selectNs(ns: string) {
    selectedNs = ns;
    detailPager = createPager({ namespace: ns, pageSize: 20, useUteke: true });
    await detailPager.loadInitial();
  }

  async function performCreateNs() {
    const name = newNsName.trim();
    if (!name) return;
    busy = true;
    try {
      await utekeServer.remember(
        `Namespace "${name}" created via CorIn — replace or delete this placeholder memory.`,
        { tags: ['namespace-placeholder'], namespace: name, metadata: { author: 'human' } },
      );
      toastStore.success(`Namespace ${name} created`);
      showNewNs = false;
      newNsName = '';
      await loadNamespaces();
      await selectNs(name);
    } catch (e) {
      toastStore.error(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      busy = false;
    }
  }

  async function performRename() {
    if (!renameTarget || !renameTo.trim()) return;
    busy = true;
    try {
      await docs.namespaceRename(renameTarget, renameTo.trim());
      toastStore.success(`Renamed to ${renameTo.trim()}`);
      const newName = renameTo.trim();
      if (selectedNs === renameTarget) await selectNs(newName);
      renameTarget = null;
      renameTo = '';
      await loadNamespaces();
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
      const res = await docs.namespaceDelete(
        deleteTarget,
        deleteStrategy,
        deleteStrategy === 'merge' ? deleteMergeTarget.trim() : undefined,
      );
      toastStore.success(`${deleteTarget}: ${res.affected} memories processed`);
      if (selectedNs === deleteTarget) {
        selectedNs = null;
        detailPager = createPager({ pageSize: 20, useUteke: true });
      }
      deleteTarget = null;
      await loadNamespaces();
    } catch (e) {
      toastStore.error(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      busy = false;
    }
  }
</script>

<div class="ns-view">
  <div class="ns-header">
    <h2>Namespaces</h2>
    <span class="count">{namespaces.length} namespace{namespaces.length === 1 ? '' : 's'}</span>
    {#if nsSupported}
      <button class="new-ns-btn" onclick={() => { showNewNs = true; newNsName = ''; }}>+ New namespace</button>
    {/if}
    {#if nsSupported === false}
      <span class="upgrade-hint" title="Namespace management needs uteke ≥ {minVersion('namespaceManage')}">
        manage needs uteke ≥ {minVersion('namespaceManage')}
      </span>
    {/if}
  </div>

  {#if loading}
    <div class="msg">Loading...</div>
  {:else}
    <div class="layout">
      <div class="ns-list">
        {#each namespaces as ns (ns.name)}
          <div
            class="ns-card"
            class:active={selectedNs === ns.name}
            role="button"
            tabindex="0"
            onclick={() => selectNs(ns.name)}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectNs(ns.name)}
          >
            <div class="ns-name">{ns.name}</div>
            <div class="ns-right">
              <span class="count">{ns.count}</span>
              {#if nsSupported}
                <button
                  class="ns-act"
                  title={isProtected(ns.name) ? 'Default workspace is protected' : 'Rename / merge'}
                  disabled={isProtected(ns.name) || busy}
                  onclick={(e) => {
                    e.stopPropagation();
                    renameTarget = ns.name;
                    renameTo = '';
                  }}
                ><ArrowRightLeft size={12} strokeWidth={2} /></button>
                <button
                  class="ns-act danger"
                  title={isProtected(ns.name) ? 'Default workspace is protected' : 'Delete (choose strategy)'}
                  disabled={isProtected(ns.name) || busy}
                  onclick={(e) => {
                    e.stopPropagation();
                    deleteTarget = ns.name;
                    deleteStrategy = 'refuse';
                  }}
                ><Trash2 size={12} strokeWidth={2} /></button>
              {/if}
            </div>
          </div>
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
            {#each detailPager.items as m (m.id)}
              <div
                class="mem-card"
                role="button"
                tabindex="0"
                onclick={() => onmemoryclick(m.id)}
                onkeydown={(e) => e.key === 'Enter' && onmemoryclick(m.id)}
              >
                <div class="mem-content">{m.content.slice(0, 120)}</div>
                <div class="mem-tags">
                  {#each m.tags.slice(0, 3) as tag}<span class="tag">{tag}</span>{/each}
                </div>
              </div>
            {/each}
          </div>
          {#if detailPager.loading && detailPager.items.length === 0}
            <div class="msg">Loading…</div>
          {/if}
          {#if detailPager.hasMore}
            <div class="load-more">
              <button onclick={() => detailPager.loadMore()} disabled={detailPager.loading}>
                {detailPager.loading ? 'Loading…' : 'Load more'}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ns-view {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .ns-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 24px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .ns-header h2 { font-size: 1.1rem; margin: 0; flex: 1; }
  .ns-header .count { font-size: 0.8rem; color: var(--text-muted); }
  .layout {
    flex: 1;
    display: flex;
    gap: 0;
    overflow: hidden;
    min-height: 0;
  }
  .ns-list {
    width: 260px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    padding: 8px 12px;
    border-right: 1px solid var(--border);
  }
  .ns-card {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; text-align: left; cursor: pointer;
    background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: var(--radius-md);
    color: var(--text-secondary); font-size: 0.85rem;
  }
  .ns-card:hover { border-color: var(--accent); }
  .ns-card.active { border-color: var(--accent); color: var(--accent); }
  .ns-name { font-weight: 500; }
  .count { color: var(--text-muted); font-size: 0.75rem; }

  .ns-detail {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 16px 24px;
  }
  .ns-detail h3 { font-size: 1rem; margin: 0 0 12px; }
  .mem-list { display: flex; flex-direction: column; gap: 8px; }
  .mem-card {
    padding: 10px 14px; cursor: pointer;
    background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: var(--radius-md);
  }
  .mem-card:hover { border-color: var(--accent); }
  .mem-content { font-size: 0.85rem; color: var(--text-primary); margin-bottom: 6px; }
  .mem-tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.7rem; padding: 2px 6px; background: var(--bg-hover); color: var(--text-secondary); border-radius: var(--radius-sm); }

  .load-more { display: flex; justify-content: center; margin-top: 16px; }
  .load-more button {
    padding: 8px 20px; cursor: pointer; font-size: 0.85rem;
    background: var(--bg-tertiary); color: var(--text-secondary);
    border: 1px solid var(--border); border-radius: var(--radius-md);
  }
  .load-more button:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }
  .load-more button:disabled { opacity: 0.6; cursor: not-allowed; }

  .msg { text-align: center; padding: 40px; color: var(--text-muted); }

  .ns-right { display: flex; align-items: center; gap: 6px; }
  .ns-act {
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 24px;
    background: transparent; border: 1px solid var(--border);
    border-radius: var(--radius-sm); color: var(--text-secondary); cursor: pointer;
  }
  .ns-act:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); }
  .ns-act:disabled { opacity: 0.4; cursor: not-allowed; }
  .ns-act.danger:hover:not(:disabled) { color: var(--red); border-color: var(--red); }
  .new-ns-btn {
    padding: 4px 12px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.78rem;
    cursor: pointer;
  }
  .new-ns-btn:hover { border-color: var(--accent); color: var(--accent); }
  .upgrade-hint {
    font-size: 0.72rem; color: var(--text-muted);
    border: 1px solid var(--border); border-radius: var(--radius-pill);
    padding: 2px 9px;
  }
  .dlg p { margin: 6px 0; font-size: 0.85rem; color: var(--text-primary); }
  .dlg code { font-family: var(--font-mono); font-size: 0.8rem; background: var(--bg-tertiary); padding: 1px 6px; border-radius: var(--radius-sm); }
  .dlg-input {
    width: 100%; padding: 6px 10px; margin: 6px 0;
    background: var(--bg-primary); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: var(--radius-md);
    font-family: var(--font-mono); font-size: 0.8rem;
  }
  .strategy { display: flex; align-items: center; gap: 8px; margin: 8px 0; font-size: 0.8rem; color: var(--text-primary); }
</style>

{#if showNewNs}
  <ConfirmDialog
    open
    title="New namespace"
    message="uteke creates a namespace together with its first memory — a small placeholder is seeded and can be replaced or deleted."
    confirmLabel={newNsName.trim() ? 'Create' : ''}
    onconfirm={performCreateNs}
    oncancel={() => (showNewNs = false)}
  >
    <div class="dlg">
      <input class="dlg-input" type="text" bind:value={newNsName} placeholder="namespace-name" />
    </div>
  </ConfirmDialog>
{/if}

{#if renameTarget}
  <ConfirmDialog
    open
    title="Rename or merge namespace?"
    message="Existing memories move with it in a single atomic update."
    confirmLabel={renameTo.trim() ? 'Apply' : ''}
    onconfirm={performRename}
    oncancel={() => (renameTarget = null)}
  >
    <div class="dlg">
      <p><code>{renameTarget}</code> →</p>
      <input class="dlg-input" type="text" bind:value={renameTo} placeholder="new-name (existing name = merge)" />
      <p class="hint">If the target exists, all memories are merged into it — no data loss.</p>
    </div>
  </ConfirmDialog>
{/if}

{#if deleteTarget}
  <ConfirmDialog
    open
    title="Delete namespace — choose strategy"
    message="A namespace is never hard-deleted — memories go somewhere safe."
    confirmLabel="Execute"
    danger
    onconfirm={performDelete}
    oncancel={() => (deleteTarget = null)}
  >
    <div class="dlg">
      <p>Namespace <code>{deleteTarget}</code> — memories must go somewhere:</p>
      <label class="strategy">
        <input type="radio" bind:group={deleteStrategy} value="refuse" />
        <span><b>Refuse</b> — cancel if any memory still uses it (safest)</span>
      </label>
      <label class="strategy">
        <input type="radio" bind:group={deleteStrategy} value="merge" />
        <span><b>Merge into</b> — move everything to:</span>
      </label>
      {#if deleteStrategy === 'merge'}
        <input class="dlg-input" type="text" bind:value={deleteMergeTarget} placeholder="target-namespace" />
      {/if}
      <label class="strategy">
        <input type="radio" bind:group={deleteStrategy} value="deprecate" />
        <span><b>Deprecate</b> — soft-delete all (restorable from Recycle Bin until TTL)</span>
      </label>
    </div>
  </ConfirmDialog>
{/if}
